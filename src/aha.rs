use super::Opt;
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
    pub fn url_builder(&self) -> Url {
        let uri = format!("https://{}.aha.io/api/v1/", self.domain);
        Url::parse(&uri).unwrap()
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

    pub fn projects(&self) -> Vec<Value> {
        let projects_url = self.url_builder().join("products?per_page=200").unwrap();
        let projects = self
            .get(projects_url, "products".to_string())
            .expect("Can not load projects. Check your domain and api keys");
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
        let releases = self
            .get(releases_url, "releases".to_string())
            .expect("Can not load release. Check your access in Aha!");
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
        let releases = self
            .get(releases_url, "features".to_string())
            .expect("Can not load features. Check your access in Aha!");
        releases.as_array().unwrap().to_vec()
    }
    pub fn send_feature(&self, feature: &FeatureCreate) {
        let uri = format!("https://{}.aha.io/api/v1/features", self.domain);
        let _response = self.client.post(&uri).json(&feature).send();
    }

    pub fn send_requirement(&self, feature_ref: String, requirement: &RequirementCreate) {
        let uri = format!(
            "https://{}.aha.io/api/v1/features/{}/requirements",
            self.domain, feature_ref
        );
        let _response = self.client.post(&uri).json(&requirement).send();
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
            if data == "No" {
                self.custom_fields = Some(CustomNotes {
                    notes: "Not required".to_string(),
                })
            }
            None
        }
    }
}

#[derive(Serialize, Debug, Deserialize)]
pub struct RequirementCreate {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<CustomNotes>,
}
impl RequirementCreate {
    pub fn new() -> RequirementCreate {
        RequirementCreate {
            name: "".to_string(),
            description: "".to_string(),
            custom_fields: None,
        }
    }
    pub fn advance(&mut self, data: String) -> Option<&str> {
        if self.name.len() == 0 {
            self.name = data;
            Some("Description")
        } else if self.description.len() == 0 {
            self.description = data;
            None
        } else {
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
