use super::github;
use super::Opt;
use notify_rust::Notification;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::process::Command;
use termion::clear;
use termion::cursor;
use text_io::read;
use url::Url;

pub struct Aha<'a> {
    pub domain: String,
    pub client: reqwest::Client,
    pub user_email: String,
    pub opt: &'a Opt,
}

impl<'a> Aha<'a> {
    pub fn generate(&self) -> Result<Value, serde_json::Error> {
        self.reset_screen();
        println!("Enter feature name:");
        let name: String = read!("{}\n");
        self.reset_screen();

        println!("Release notes?:");
        println!("0) No");
        println!("1) Yes");
        let notes: i8 = read!();
        self.reset_screen();

        let mut feature = self.create_feature(name, notes).unwrap()["feature"].take();

        let feature_url = self
            .url_builder()
            .join("features/")
            .unwrap()
            .join(feature["id"].as_str().unwrap())
            .unwrap();
        let description = feature["feature"]["description"]["body"].take();
        if !description.is_null() {
            {
                let mut file = File::create("/tmp/rust-workflow").unwrap();
                file.write_all(description.to_string().as_bytes()).unwrap();
            }
        }
        Command::new("nvim")
            .arg("/tmp/rust-workflow")
            .status()
            .expect("Something went wrong.");
        let file = File::open("/tmp/rust-workflow").unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();

        let update = FeatureUpdateCreate {
            description: Some(contents),
            assigned_to_user: Some(self.user_email.clone()),
            custom_fields: None,
            workflow_status: Some(WorkflowStatusUpdate {
                name: "In development".to_string(),
            }),
        };

        let json_string = serde_json::to_string(&update)?;

        if self.opt.verbose {
            println!("puting json: {}", json_string);
        }
        let response = self
            .client
            .put(&feature_url.to_string())
            .json(&update)
            .send();
        let content = response.unwrap().text();
        let text = &content.unwrap_or("".to_string());
        if self.opt.verbose {
            println!("updated {:?}", text);
        }
        let feature: Result<Value, _> = serde_json::from_str(&text);

        if let Ok(f) = feature {
            Ok(f)
        } else {
            println!("json failed to parse {:?}", text);
            let ex: Result<_, serde_json::Error> = Err(feature.unwrap_err());
            ex
        }
    }
    pub fn url_builder(&self) -> Url {
        let uri = format!("https://{}.aha.io/api/v1/", self.domain);
        Url::parse(&uri).unwrap()
    }

    pub fn status_for_labels(
        &self,
        labels: Vec<String>,
        config_labels: Option<HashMap<String, String>>,
    ) -> Option<String> {
        let mut default_labels = HashMap::new();
        default_labels.insert("In development".to_string(), "In development".to_string());
        default_labels.insert(
            "Needs code review".to_string(),
            "In code review".to_string(),
        );
        default_labels.insert("Needs PM review".to_string(), "In PM review".to_string());
        default_labels.insert("Ready".to_string(), "Ready to ship".to_string());
        labels
            .iter()
            .map(|label| {
                let default = default_labels.get(label);
                let x = match &config_labels {
                    Some(c) => c.get(label).or_else(|| default),
                    None => default,
                };
                match x {
                    Some(c) => Some(c.clone()),
                    None => None,
                }
            })
            .filter(|label| label.is_some())
            .nth(1)
            .unwrap_or(None)
    }
    pub fn new(domain: String, auth_token: String, email: String, opt: &Opt) -> Aha {
        let mut headers = reqwest::header::HeaderMap::new();
        let mut auth =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", auth_token)).unwrap();
        auth.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth);
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("Rust aha api v1 (Becker@aha.io)"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let client = reqwest::Client::builder()
            .gzip(true)
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(50))
            .build()
            .unwrap();
        Aha {
            client,
            domain,
            user_email: email,
            opt,
        }
    }

    pub fn sync_pr(
        &self,
        pr: github::PullRequest,
        labels: Option<HashMap<String, String>>,
    ) -> Result<(), failure::Error> {
        if let Some((source, key)) = self.type_from_name(&pr.name) {
            if self.opt.verbose {
                println!("matched {} {} {}", pr.name, source, key);
            }

            match self.get_json(key.clone(), source.to_string()) {
                Ok(feature) => self.update_aha(key, pr, feature, labels, source).unwrap(),
                Err(error) => println!("Error {}: {}", source, error),
            }
        } else if self.opt.verbose {
            println!("Did not match {}", pr.name);
        }
        Ok(())
    }
    pub fn generate_update_function(
        &self,
        current: &Value,
        pr: &github::PullRequest,
        status: Option<String>,
    ) -> FeatureUpdate {
        let assigned = if current["assigned_to_user"].is_null() {
            Some(self.user_email.clone())
        } else {
            None
        };
        let count = if current["custom_fields"].is_null() {
            0
        } else {
            current["custom_fields"]
                .as_array()
                .unwrap()
                .iter()
                .by_ref()
                .filter(|cf| cf["name"] == "Pull Request")
                .count()
        };
        // set it if it is not there.
        let custom = if count == 0 {
            Some(CustomFieldGithub {
                github_url: pr.url.clone(),
            })
        } else {
            None
        };

        let mut status = if let Some(wf) = status {
            Some(WorkflowStatusUpdate { name: wf })
        } else {
            None
        };
        let current_status = &current["workflow_status"]["name"];
        if status.is_none()
            && (current_status == "Ready to develop" || current_status == "Under consideration")
        {
            status = Some(WorkflowStatusUpdate {
                name: "In code review".to_string(),
            })
        }

        FeatureUpdate {
            assigned_to_user: assigned,
            custom_fields: custom,
            workflow_status: status,
        }
    }
    pub fn reset_screen(&self) {
        if !self.opt.verbose {
            print!(
                "{clear}{goto}",
                clear = clear::All,
                goto = cursor::Goto(1, 1)
            );
        }
    }

    pub fn projects(&self) -> Vec<Value> {
        let projects_url = self.url_builder().join("products?per_page=200").unwrap();
        let projects = self.get(projects_url, "products".to_string()).unwrap();
        projects.as_array().unwrap().to_vec()
    }
    pub fn releases(&self, project_id: String) -> Vec<Value> {
        let releases_url = self
            .url_builder()
            .join("products/")
            .unwrap()
            .join(&format!("{}/", project_id))
            .unwrap()
            .join("releases?exclude_shipped=true&per_page=200")
            .unwrap();
        let releases = self.get(releases_url, "releases".to_string()).unwrap();
        releases.as_array().unwrap().to_vec()
    }

    pub fn features(&self, project_id: String) -> Vec<Value> {
        let releases_url = self
            .url_builder()
            .join("releases/")
            .unwrap()
            .join(&format!("{}/", project_id))
            .unwrap()
            .join("features?per_page=200&fields=*")
            .unwrap();
        let releases = self.get(releases_url, "features".to_string()).unwrap();
        releases.as_array().unwrap().to_vec()
    }
    pub fn send_feature(&self, feature: &FeatureCreate) {
        let uri = format!("https://{}.aha.io/api/v1/features", self.domain);
        let _json_string = serde_json::to_string(&feature).unwrap();
        let _response = self.client.post(&uri).json(&feature).send();
    }

    pub fn create_feature(&self, name: String, notes: i8) -> Result<Value, serde_json::Error> {
        let projects_url = self.url_builder().join("products?per_page=200").unwrap();
        let projects = self.get(projects_url, "products".to_string()).unwrap();
        let projects = projects.as_array().unwrap();
        for (index, project) in projects.iter().enumerate() {
            println!("{}) {} ({})", index, project["name"], project["id"]);
        }
        println!("Choose a product:");
        let index: usize = read!();
        self.reset_screen();

        let releases_url = self
            .url_builder()
            .join("products/")
            .unwrap()
            .join(&format!("{}/", projects[index]["id"].as_str().unwrap()))
            .unwrap()
            .join("releases?exclude_shipped=true&per_page=200")
            .unwrap();
        let releases = self.get(releases_url, "releases".to_string()).unwrap();
        let releases = releases.as_array().unwrap();
        for (index, release) in releases.iter().enumerate() {
            println!("{}) {} ({})", index, release["name"], release["id"]);
        }
        println!("Choose a release:");
        let index: usize = read!();
        self.reset_screen();

        let uri = format!("https://{}.aha.io/api/v1/features", self.domain);
        let notes_required = if notes == 1 {
            Some(CustomNotes {
                notes: "Required".to_string(),
            })
        } else {
            None
        };

        let feature = FeatureCreate {
            name,
            description: "".to_string(),
            release_id: releases[index]["id"].as_str().unwrap().to_string(),
            custom_fields: notes_required,
        };
        let json_string = serde_json::to_string(&feature)?;
        if self.opt.verbose {
            println!("creating feature json: {}", json_string);
        }
        let response = self.client.post(&uri).json(&feature).send();
        let content = response.unwrap().text();
        let text = &content.unwrap_or("".to_string());
        if self.opt.verbose {
            println!("created {:?}", text);
        }

        serde_json::from_str(&text)
    }

    pub fn update_aha(
        &self,
        key: String,
        pr: github::PullRequest,
        current: Value,
        labels: Option<HashMap<String, String>>,
        base: String,
    ) -> Result<(), serde_json::Error> {
        let uri = format!("https://{}.aha.io/api/v1/{}s/{}", self.domain, base, key);
        let status = self.status_for_labels(pr.labels.clone(), labels);
        let feature = self.generate_update_function(&current, &pr, status);
        let json_string = serde_json::to_string(&feature)?;
        if self.opt.verbose {
            println!("puting {} json: {} | {}", base, json_string, uri);
        }
        if !self.opt.silent && json_string.len() > 4 && !current["url"].is_null() {
            Notification::new()
                .summary(&format!("Updating requirement {}", key))
                .body(&format!(
                    "{}\n{}",
                    current["url"].as_str().unwrap(),
                    pr.number
                ))
                .icon("firefox")
                .timeout(0)
                .show()
                .unwrap();
        }
        if !self.opt.dry_run && json_string.len() > 4 {
            let response = self.client.put(&uri).json(&feature).send();
            let content = response.unwrap().text();
            let text = &content.unwrap_or("".to_string());
            if self.opt.verbose {
                println!("updated {} {:?}", base, text);
            }
            let feature: Result<Value, _> = serde_json::from_str(&text);

            if let Ok(f) = feature {
                if f[base].is_null() {
                    println!("json failed to parse {:?}", text);
                }
                Ok(())
            } else {
                if self.opt.verbose {
                    println!("json failed to parse {:?}", text);
                }
                let ex: Result<(), serde_json::Error> = Err(feature.unwrap_err());
                ex
            }
        } else {
            Ok(())
        }
    }

    pub fn type_from_name(&self, name: &str) -> Option<(String, String)> {
        //could return enum
        let req = Regex::new(r"^([A-Z]+-\d+-\d+)").unwrap();
        let fet = Regex::new(r"^([A-Z]{1,}-\d{1,})").unwrap();
        let rc = req.captures(&name.trim());
        let fc = fet.captures(&name.trim());
        if let Some(rc) = rc {
            Some(("requirement".to_string(), rc[0].to_string()))
        } else if let Some(fc) = fc {
            Some(("feature".to_string(), fc[0].to_string()))
        } else {
            None
        }
    }

    pub fn get(&self, url: Url, base: String) -> Result<Value, serde_json::Error> {
        let uri = url.to_string();
        if self.opt.verbose {
            println!("{} url: {}", base, uri);
        }
        let response = self.client.get(&uri).send();
        let content = response.unwrap().text();
        if self.opt.verbose {
            println!("{} text {:?}", base, content);
        }
        let feature: Result<Value, _> = serde_json::from_str(&content.unwrap_or("".to_string()));
        if let Ok(mut fe) = feature {
            Ok(fe[base].take())
        } else {
            let ex: Result<Value, serde_json::Error> = Err(feature.unwrap_err());
            ex
        }
    }

    pub fn get_json(&self, end_path: String, base: String) -> Result<Value, serde_json::Error> {
        let uri = format!("https://{}.aha.io/api/v1/", self.domain);
        let url = Url::parse(&uri).unwrap();

        let api_url = if !end_path.is_empty() {
            format!("/{}", end_path)
        } else {
            "".to_string()
        };
        let url = url.join(&format!("{}{}{}", base, "s", api_url)).unwrap();
        self.get(url, base)
    }
}

// keep
#[derive(Serialize, Debug, Deserialize)]
pub struct FeatureCreate {
    pub name: String,
    pub description: String,
    pub release_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<CustomNotes>,
}
impl FeatureCreate {
    pub fn new() -> FeatureCreate {
        FeatureCreate {
            name: "".to_string(),
            description: "".to_string(),
            release_id: "".to_string(),
            custom_fields: None,
        }
    }
    pub fn advance(&mut self, data: String) -> Option<&str> {
        if self.name.len() == 0 {
            self.name = data;
            Some("Description")
        } else if self.description.len() == 0 {
            self.description = data;
            Some("Needs notes? (Yes/No)")
        } else {
            if data == "Yes" {
                self.custom_fields = Some(CustomNotes {
                    notes: "Required".to_string(),
                })
            }
            None
        }
    }
}

// keep
#[derive(Serialize, Debug, Deserialize)]
pub struct FeatureUpdateCreate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<CustomFieldGithub>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_status: Option<WorkflowStatusUpdate>,
}

// keep
#[derive(Serialize, Debug, Deserialize)]
pub struct FeatureUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    assigned_to_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_fields: Option<CustomFieldGithub>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_status: Option<WorkflowStatusUpdate>,
}
//keep
#[derive(Serialize, Debug, Deserialize)]
pub struct WorkflowStatusUpdate {
    pub name: String,
}

// kepp
#[derive(Serialize, Debug, Deserialize)]
pub struct CustomNotes {
    #[serde(rename = "release_notes1")]
    notes: String,
}
// kepp
#[derive(Serialize, Debug, Deserialize)]
pub struct CustomFieldGithub {
    #[serde(rename = "pull_request")]
    github_url: String,
}
