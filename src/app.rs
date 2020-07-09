use super::key_layout::KeyLayout;
use super::util::StatefulList;
use super::Aha;

use html2md;
use scarlet::color::RGBColor;

use super::util::event::Event;
use serde_json::Value;

use dirs;
use slog;

use slog::Drain;
use std::fs::OpenOptions;

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use termion::event::Key;
use tui::{
    style::{Color, Modifier, Style},
    widgets::Text,
};
#[derive(PartialEq, Deserialize, Serialize, Clone)]
pub enum Screen {
    Project,
    Release,
    Features,
    Feature,
    Requirement,
    Creating,
    Search,
}

// auto select the menus based on last view
#[derive(Deserialize, Serialize, Clone)]
pub struct History {
    pub project: Option<String>,
    pub release: Option<String>,
    pub feature: Option<String>,
}

// auto select the menus based on last view
#[derive(Deserialize, Serialize, Clone)]
pub struct Layout {
    pub up: Option<String>,
    pub down: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub up_arrow: Option<String>,
    pub down_arrow: Option<String>,
    pub left_arrow: Option<String>,
    pub right_arrow: Option<String>,
    pub right_alt: Option<String>,
    pub escape: Option<String>,
    pub quit: Option<String>,
    pub search: Option<String>,
    pub create: Option<String>,
}

#[derive(PartialEq)]
pub enum Popup {
    Text,
    None,
    Search,
}
use super::aha::{FeatureCreate, RequirementCreate};
pub struct App<'a> {
    pub layout: KeyLayout,
    pub logger: slog::Logger,
    pub items: StatefulList<(String, Value)>,
    pub releases: StatefulList<(String, Value)>,
    pub features: StatefulList<(String, Value)>,
    pub feature_text: Vec<String>,
    pub feature_title: String,
    pub debug_txt: String,
    pub feature_text_formatted: Option<Vec<Text<'a>>>,
    pub active_layer: Screen,
    pub popup: Popup,
    pub text_box: String,
    pub text_box_title: String,
    pub new_feature: FeatureCreate,
    pub new_requirement: RequirementCreate,
    pub events: Vec<(&'a str, &'a str)>,
    pub info_style: Style,
    pub warning_style: Style,
    pub error_style: Style,
    pub critical_style: Style,
    pub history: Option<History>,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let log_path = "/tmp/ahacli.log";
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .unwrap();

        let decorator = slog_term::PlainDecorator::new(file);
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();

        let log = slog::Logger::root(drain, o!());
        App {
            layout: KeyLayout::default(),
            logger: log,
            popup: Popup::None,
            items: StatefulList::with_items(vec![]),
            releases: StatefulList::with_items(vec![]),
            features: StatefulList::with_items(vec![]),
            feature_text: vec!["".to_string()],
            feature_text_formatted: None,
            history: None,
            debug_txt: "".to_string(),
            feature_title: "".to_string(),
            active_layer: Screen::Project,
            new_feature: FeatureCreate::new(),
            new_requirement: RequirementCreate::new(),
            text_box: "".to_string(),
            text_box_title: "Feature Name".to_string(),
            events: vec![("Event1", "INFO")],
            info_style: Style::default().fg(Color::White),
            warning_style: Style::default().fg(Color::Yellow),
            error_style: Style::default().fg(Color::Magenta),
            critical_style: Style::default().fg(Color::Red),
        }
    }

    pub fn load_features(&mut self, release_id: String, aha: &Aha) {
        let feature_list = aha.features(release_id.clone());

        self.write_history("release".to_string(), release_id);
        self.features = StatefulList::with_items(
            feature_list
                .iter()
                .map(|project| {
                    let mut vec = vec![];

                    vec.push((
                        format!(
                            "{} - {}",
                            project["name"].as_str().unwrap(),
                            project["workflow_status"]["name"].as_str().unwrap(),
                        ),
                        project.clone(),
                    ));
                    if let Some(reqs) = project["requirements"].as_array() {
                        let last = reqs.len();
                        for (i, req) in reqs.iter().enumerate() {
                            let brace = if i == last - 1 { "└" } else { "├" };
                            vec.push((
                                format!(
                                    "{} {} - {}",
                                    brace,
                                    req["name"].as_str().unwrap(),
                                    req["workflow_status"]["name"].as_str().unwrap(),
                                ),
                                project.clone(),
                            ))
                        }
                    }
                    vec
                })
                .flatten()
                .collect(),
        );
    }

    pub fn load_releases(&mut self, project_id: String, aha: &Aha) {
        let releases = aha.releases(project_id.clone());
        self.write_history("project".to_string(), project_id);
        self.releases = StatefulList::with_items(
            releases
                .iter()
                .map(|project| {
                    (
                        project["name"].as_str().unwrap().to_string(),
                        project.clone(),
                    )
                })
                .collect(),
        );
    }
    pub fn write_history(&mut self, key: String, value: String) {
        if self.history.is_none() {
            self.history = Some(History {
                project: None,
                release: None,
                feature: None,
            });
        }
        match key.as_str() {
            "project" => {
                self.history.as_mut().unwrap().project = Some(value);
            }
            "release" => {
                self.history.as_mut().unwrap().release = Some(value);
            }
            "feature" => {
                self.history.as_mut().unwrap().feature = Some(value);
            }
            _ => {}
        }

        let clean_string = toml::to_string(&self.history.as_ref().unwrap()).unwrap();
        let home_dir = dirs::home_dir().expect("Could not find home path");
        let path_name = format!("{}/.aha_cli_cache", home_dir.display());
        let path = Path::new(&path_name);
        match File::create(&path) {
            Err(why) => {
                self.debug_txt = format!("couldn't create {}: {}", path_name, why);
            }
            Ok(mut file) => match file.write_all(clean_string.as_bytes()) {
                Err(why) => {
                    self.debug_txt = format!("couldn't write to {}: {}", path_name, why);
                }
                Ok(_) => {}
            },
        };
    }

    fn get_key_from(&self, input: &str) -> Key {
        if input.len() == 1 {
            Key::Char(input.as_bytes()[0] as char)
        } else if input == "up" {
            Key::Up
        } else if input == "down" {
            Key::Down
        } else if input == "left" {
            Key::Left
        } else if input == "right" {
            Key::Right
        } else if input == "esc" {
            Key::Esc
        } else if input == "none" {
            Key::Null
        } else if input == "\n" {
            Key::Char('\n')
        } else if input.starts_with("alt+") {
            let text = input.splitn(2, "+").collect::<Vec<_>>();
            let text = text.last().unwrap();
            Key::Alt(text.as_bytes()[0] as char)
        } else if input.starts_with("ctrl+") {
            let text = input.splitn(2, "+").collect::<Vec<_>>();
            let text = text.last().unwrap();
            Key::Ctrl(text.as_bytes()[0] as char)
        } else {
            Key::Null
        }
    }

    pub fn load_layout(&mut self, file: String) {
        let value: Layout = toml::from_str(&file).unwrap();
        if let Some(x) = value.up {
            self.layout.up = self.get_key_from(&x);
        }
    }

    pub fn load_history(&mut self, file: String, aha: &Aha) {
        let value: History = toml::from_str(&file).unwrap();
        let return_value = value.clone();
        if let Some(project) = value.project {
            let project = project.to_string();
            if let Some(index) = self.items.items.iter().position(|x| x.1["id"] == project) {
                let project_id = self.items.items[index].1["id"]
                    .as_str()
                    .unwrap()
                    .to_string();
                self.items.state.select(Some(index));
                self.load_releases(project_id, &aha);

                self.active_layer = Screen::Release;
                if let Some(release) = value.release {
                    if let Some(index) = self
                        .releases
                        .items
                        .iter()
                        .position(|x| x.1["id"] == release)
                    {
                        let release_id = self.releases.items[index].1["id"]
                            .as_str()
                            .unwrap()
                            .to_string();

                        self.releases.state.select(Some(index));
                        self.load_features(release_id, &aha);

                        self.active_layer = Screen::Features;
                    }

                    if let Some(feature) = value.feature {
                        if let Some(index) = self
                            .features
                            .items
                            .iter()
                            .position(|x| x.1["id"] == feature)
                        {
                            // todo: need to format the selected feature and write the history
                            self.active_layer = Screen::Features;
                            self.features.state.select(Some(index));
                        }
                    }
                }
            }
        }

        self.history = Some(return_value);
    }
    pub fn help_text(&mut self) {
        if self.active_layer != Screen::Feature {
            let mut base = vec![
                Text::raw("Poor instructions\n"),
                Text::raw("\n===================\n"),
                Text::raw("j ↑ - up a list\n"),
                Text::raw("k ↓ - down a list down\n"),
                Text::raw("l → enter - enter selected item\n"),
                Text::raw("h ← - previous section\n"),
                Text::raw("q - exit\n"),
                Text::raw("esc - to close popups\n"),
            ];
            if self.active_layer != Screen::Project {
                base.push(Text::raw("\nRelease Actions:\n"));
                base.push(Text::raw("c - create feature if a release is selected.\n"));
                base.push(Text::raw(
                    "c - create requirement if a feature is selected.\n",
                ));
            }
            self.feature_text_formatted = Some(base);
        }
    }

    pub fn advance(&mut self) {
        let event = self.events.pop().unwrap();
        self.events.insert(0, event);
    }

    pub fn format_selected_feature(
        &mut self,
        max_width: usize,
    ) -> std::vec::Vec<tui::widgets::Text<'_>> {
        if self.active_layer == Screen::Feature {
            match self.features.state.selected() {
                Some(i) => {
                    if let Some(data) = self.feature_text_formatted.as_ref() {
                        data.clone()
                    } else {
                        let feature = self.features.items[i].clone();
                        let selected_feature = if feature.0.starts_with("└")
                            || feature.0.starts_with("├")
                        {
                            let clean_string = feature
                                .0
                                .splitn(2, " ")
                                .collect::<Vec<_>>()
                                .last()
                                .unwrap()
                                .to_string();
                            let requirement = feature.1["requirements"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .find({
                                    |req| {
                                        clean_string
                                            == format!(
                                                "{} - {}",
                                                req["name"].as_str().unwrap(),
                                                req["workflow_status"]["name"].as_str().unwrap(),
                                            )
                                    }
                                })
                                .unwrap_or(&feature.1)
                                .clone();
                            self.feature_title = format!(
                                "Requirement {}",
                                requirement["reference_num"].as_str().unwrap()
                            );
                            requirement
                        } else {
                            self.feature_title =
                                format!("Feature {}", feature.1["reference_num"].as_str().unwrap());
                            feature.1.clone()
                        };
                        self.feature_text = vec![selected_feature.to_string()];
                        let rgb1 = RGBColor::from_hex_code(
                            selected_feature["workflow_status"]["color"]
                                .as_str()
                                .unwrap(),
                        )
                        .unwrap()
                        .int_rgb_tup();
                        //self.debug_txt = format!("{:?}", rgb1);

                        let html = selected_feature["description"]["body"]
                            .as_str()
                            .unwrap()
                            .to_string();
                        //self.debug_txt = format!("{:?} - {} - {}", rgb1, max_width, max_width - 9);
                        let width = if max_width % 2 == 0 {
                            max_width - 8
                        } else {
                            max_width - 9
                        };

                        let markdown = html2md::parse_html_custom(
                            &html,
                            &HashMap::default(),
                            html2md::Config {
                                max_length: width,
                                new_line_break: "\n".to_string(),
                                logger: None,
                            },
                        );
                        let result = vec![
                            Text::raw(selected_feature["name"].as_str().unwrap().to_string()),
                            Text::raw(" ["),
                            Text::styled(
                                selected_feature["workflow_status"]["name"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                                Style::default().bg(Color::Rgb(
                                    rgb1.0 as u8,
                                    rgb1.1 as u8,
                                    rgb1.2 as u8,
                                )),
                            ),
                            Text::raw("]\n"),
                            Text::raw(
                                selected_feature["assigned_to_user"]["name"]
                                    .as_str()
                                    .unwrap_or("Unassigned")
                                    .to_string(),
                            ),
                            Text::raw("\n"),
                            Text::raw(selected_feature["url"].as_str().unwrap().to_string()),
                            Text::raw("\n"),
                            Text::raw("\n"),
                            Text::raw(markdown),
                        ];
                        self.feature_text_formatted = Some(result.clone());
                        result
                    }
                }
                None => vec![],
            }
        } else {
            self.help_text();
            if let Some(data) = self.feature_text_formatted.as_ref() {
                data.clone()
            } else {
                vec![]
            }
        }
    }

    pub fn handle_search_popup(&mut self, event: Event<Key>, aha: &Aha) -> Option<()> {
        match event {
            Event::Input(input) => {
                if input == self.layout.escape {
                    //hide
                    self.popup = Popup::None;
                } else if input == Key::Char('\n') {
                } else if input == Key::Backspace {
                    self.text_box.pop();
                } else if let Key::Char(c) = input {
                    self.text_box.push(c);
                }
            }
            Event::Tick => {
                self.advance();
            }
        }
        //dont break from here
        Some(())
    }

    pub fn handle_create_requirement_popup(&mut self, event: Event<Key>, aha: &Aha) -> Option<()> {
        match event {
            Event::Input(input) => {
                if input == self.layout.escape {
                    self.popup = Popup::None;
                    self.new_requirement = RequirementCreate::new();
                    self.text_box_title = "Requirement Name".to_string();
                } else if input == Key::Char('\n') {
                    self.debug_txt = format!("enter");
                    if let Some(title) = self.new_requirement.advance(self.text_box.to_string()) {
                        self.text_box_title = title.to_string();

                        self.text_box = "".to_string();
                    } else {
                        self.debug_txt = format!("sending requirement");
                        self.popup = Popup::None;
                        self.text_box = "".to_string();
                        // send
                        let i = self.releases.state.selected().unwrap();
                        let project = self.releases.items[i].clone();
                        let i = self.features.state.selected().unwrap();
                        let feature = self.features.items[i].clone();
                        let feature_ref = feature.1["reference_num"].as_str().unwrap().to_string();
                        aha.send_requirement(feature_ref, &self.new_requirement);

                        self.load_features(project.1["id"].as_str().unwrap().to_string(), &aha);

                        self.debug_txt = format!("requirement created");
                        self.new_requirement = RequirementCreate::new();
                        self.text_box_title = "Requirement Name".to_string();
                    }
                } else if input == Key::Backspace {
                    self.text_box.pop();
                } else if let Key::Char(c) = input {
                    self.debug_txt = format!("char {}", c);
                    self.text_box.push(c);
                }
            }
            Event::Tick => {
                self.advance();
            }
        }
        //dont break from here
        Some(())
    }
    pub fn handle_create_popup(&mut self, event: Event<Key>, aha: &Aha) -> Option<()> {
        match event {
            Event::Input(input) => {
                if input == self.layout.escape {
                    self.popup = Popup::None;
                    self.new_feature = FeatureCreate::new();
                    self.text_box_title = "Feature Name".to_string();
                } else if input == Key::Char('\n') {
                    self.debug_txt = format!("enter");
                    if let Some(title) = self.new_feature.advance(self.text_box.to_string()) {
                        self.text_box_title = title.to_string();

                        self.text_box = "".to_string();
                    } else {
                        self.debug_txt = format!("sending feature");
                        self.popup = Popup::None;
                        self.text_box = "".to_string();
                        // send
                        let i = self.releases.state.selected().unwrap();
                        let project = self.releases.items[i].clone();
                        self.new_feature.release_id = project.1["id"].as_str().unwrap().to_string();
                        aha.send_feature(&self.new_feature);

                        self.load_features(project.1["id"].as_str().unwrap().to_string(), &aha);

                        self.debug_txt = format!("feature created");
                        self.new_feature = FeatureCreate::new();
                        self.text_box_title = "Feature Name".to_string();
                    }
                } else if input == Key::Backspace {
                    self.text_box.pop();
                } else if let Key::Char(c) = input {
                    self.debug_txt = format!("char {}", c);
                    self.text_box.push(c);
                }
            }

            Event::Tick => {
                self.advance();
            }
        }
        //dont break from here
        Some(())
    }

    pub fn handle_nav(&mut self, event: Event<Key>, aha: &Aha) -> Option<()> {
        match event {
            Event::Input(input) => {
                if input == self.layout.quit {
                    self.debug_txt = format!("q exit");
                    None
                } else if input == self.layout.search {
                    self.debug_txt = format!("search");
                    self.popup = Popup::Search;
                    Some(())
                } else if input == self.layout.create {
                    self.debug_txt = format!("create");
                    if self.active_layer == Screen::Feature {
                        self.text_box_title = "Requirement Name".to_string();
                    } else {
                        self.text_box_title = "Feature Name".to_string();
                    }
                    self.popup = Popup::Text;
                    Some(())
                } else if input == self.layout.left || input == self.layout.left_arrow {
                    self.feature_text_formatted = None;
                    self.debug_txt = format!("back");
                    if self.active_layer == Screen::Project {}
                    if self.active_layer == Screen::Release {
                        self.releases.unselect();
                        self.active_layer = Screen::Project;
                    }

                    if self.active_layer == Screen::Features {
                        self.features.unselect();
                        self.active_layer = Screen::Release;
                    }

                    if self.active_layer == Screen::Feature {
                        self.active_layer = Screen::Features;
                    }

                    Some(())
                } else if input == self.layout.right
                    || input == self.layout.right_arrow
                    || input == self.layout.right_alt
                {
                    self.feature_text_formatted = None;
                    self.debug_txt = format!("over");
                    if self.active_layer == Screen::Features {
                        if self.features.state.selected().is_some() {
                            self.active_layer = Screen::Feature;
                        };
                    }
                    if self.active_layer == Screen::Release {
                        match self.releases.state.selected() {
                            Some(i) => {
                                self.active_layer = Screen::Features;
                                let release = self.releases.items[i].clone();

                                self.load_features(
                                    release.1["id"].as_str().unwrap().to_string(),
                                    &aha,
                                );
                            }
                            None => {}
                        };
                    }
                    if self.active_layer == Screen::Project {
                        match self.items.state.selected() {
                            Some(i) => {
                                self.active_layer = Screen::Release;
                                let project = self.items.items[i].clone();
                                self.load_releases(
                                    project.1["id"].as_str().unwrap().to_string(),
                                    &aha,
                                );
                            }
                            None => {}
                        };
                    }

                    Some(())
                } else if input == self.layout.down || input == self.layout.down_arrow {
                    self.feature_text_formatted = None;
                    self.debug_txt = format!("down");
                    match self.active_layer {
                        Screen::Project => self.items.next(),
                        Screen::Release => self.releases.next(),
                        Screen::Features => self.features.next(),
                        Screen::Feature => self.features.next(),
                        _ => {}
                    }

                    Some(())
                } else if input == self.layout.up || input == self.layout.up_arrow {
                    self.feature_text_formatted = None;
                    self.debug_txt = format!("up");
                    match self.active_layer {
                        Screen::Project => self.items.previous(),
                        Screen::Release => self.releases.previous(),
                        Screen::Features => self.features.previous(),
                        Screen::Feature => self.features.previous(),
                        _ => {}
                    }

                    Some(())
                } else {
                    self.debug_txt = format!("{:?}", input);
                    Some(())
                }
            }

            Event::Tick => {
                self.advance();
                Some(())
            }
        }
    }
}
