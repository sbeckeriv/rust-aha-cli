use super::util::StatefulList;
use super::Aha;

use scarlet::color::RGBColor;

use super::util::event::Event;
use serde_json::Value;

use termion::event::Key;
use tui::{
    style::{Color, Modifier, Style},
    widgets::Text,
};
#[derive(PartialEq)]
pub enum Popup {
    Text,
    None,
    Search,
}
use super::aha::FeatureCreate;
pub struct App<'a> {
    pub items: StatefulList<(String, Value)>,
    pub releases: StatefulList<(String, Value)>,
    pub features: StatefulList<(String, Value)>,
    pub feature_text: Vec<String>,
    pub debug_txt: String,
    pub feature_text_formatted: Vec<Text<'a>>,
    pub active_layer: i8,
    pub popup: Popup,
    pub text_box: String,
    pub text_box_title: String,
    pub new_feature: FeatureCreate,
    pub events: Vec<(&'a str, &'a str)>,
    pub info_style: Style,
    pub warning_style: Style,
    pub error_style: Style,
    pub critical_style: Style,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let mut app = App {
            popup: Popup::None,
            items: StatefulList::with_items(vec![]),
            releases: StatefulList::with_items(vec![]),
            features: StatefulList::with_items(vec![]),
            feature_text: vec!["".to_string()],
            feature_text_formatted: vec![],
            debug_txt: "".to_string(),
            active_layer: 0,
            new_feature: FeatureCreate::new(),
            text_box: "".to_string(),
            text_box_title: "Feature Name".to_string(),
            events: vec![("Event1", "INFO")],
            info_style: Style::default().fg(Color::White),
            warning_style: Style::default().fg(Color::Yellow),
            error_style: Style::default().fg(Color::Magenta),
            critical_style: Style::default().fg(Color::Red),
        };
        app
    }
    pub fn help_text(&mut self) {
        if self.active_layer != 3 {
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
            if self.active_layer > 0 {
                base.push(Text::raw("\nRelease Actions:\n"));
                base.push(Text::raw("c - create feature if release selected.\n"));
            }
            self.feature_text_formatted = base;
        }
    }

    pub fn advance(&mut self) {
        let event = self.events.pop().unwrap();
        self.events.insert(0, event);
    }

    pub fn format_selected_feature(&mut self) {
        match self.features.state.selected() {
            Some(i) => {
                self.active_layer = 3;
                let feature = self.features.items[i].clone();
                self.feature_text = vec![feature.1.to_string()];
                let rgb1 = RGBColor::from_hex_code(
                    feature.1["workflow_status"]["color"].as_str().unwrap(),
                )
                .unwrap()
                .int_rgb_tup();
                self.debug_txt = format!("{:?}", rgb1);
                self.feature_text_formatted = vec![
                    Text::styled(
                        feature.1["reference_num"].as_str().unwrap().to_string(),
                        Style::default().modifier(Modifier::BOLD),
                    ),
                    Text::raw(" "),
                    Text::raw(feature.1["name"].as_str().unwrap().to_string()),
                    Text::raw(" ["),
                    Text::styled(
                        feature.1["workflow_status"]["name"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                        Style::default().bg(Color::Rgb(rgb1.0 as u8, rgb1.1 as u8, rgb1.2 as u8)),
                    ),
                    Text::raw("]\n"),
                    Text::raw(
                        feature.1["assigned_to_user"]["name"]
                            .as_str()
                            .unwrap_or("Unassigned")
                            .to_string(),
                    ),
                    Text::raw("\n"),
                    Text::raw(feature.1["url"].as_str().unwrap().to_string()),
                    Text::raw("\n"),
                    Text::raw(
                        feature.1["description"]["body"]
                            .as_str()
                            .unwrap()
                            .to_string(),
                    ),
                ];
            }
            None => {}
        };
    }

    pub fn handle_search_popup(&mut self, event: Event<Key>, aha: &Aha) -> Option<()> {
        match event {
            Event::Input(input) => match input {
                Key::Esc => {
                    //hide
                    self.popup = Popup::None;
                }

                Key::Char('\n') => {}

                Key::Char(c) => {
                    self.text_box.push(c);
                }

                Key::Backspace => {
                    self.text_box.pop();
                }
                _ => {

                    //no opt for arrow keys
                }
            },
            Event::Tick => {
                self.advance();
            }
        }
        //dont break from here
        Some(())
    }
    pub fn handle_create_popup(&mut self, event: Event<Key>, aha: &Aha) -> Option<()> {
        match event {
            Event::Input(input) => match input {
                Key::Esc => {
                    //hide
                    self.popup = Popup::None;

                    self.new_feature = FeatureCreate::new();
                    self.text_box_title = "Feature Name".to_string();
                }

                Key::Char('\n') => {
                    if let Some(title) = self.new_feature.advance(self.text_box.to_string()) {
                        self.text_box_title = title.to_string();

                        self.text_box = "".to_string();
                    } else {
                        self.popup = Popup::None;
                        self.text_box = "".to_string();
                        // send
                        let i = self.releases.state.selected().unwrap();
                        let project = self.releases.items[i].clone();
                        self.new_feature.release_id = project.1["id"].as_str().unwrap().to_string();
                        aha.send_feature(&self.new_feature);

                        let releases = aha.features(project.1["id"].as_str().unwrap().to_string());

                        self.features = StatefulList::with_items(
                            releases
                                .iter()
                                .map(|project| {
                                    (
                                        format!(
                                            "{} - {}",
                                            project["name"], project["workflow_status"]["name"]
                                        ),
                                        project.clone(),
                                    )
                                })
                                .collect(),
                        );

                        self.new_feature = FeatureCreate::new();
                        self.text_box_title = "Feature Name".to_string();
                    }
                }

                Key::Char(c) => {
                    self.text_box.push(c);
                }

                Key::Backspace => {
                    self.text_box.pop();
                }
                _ => {

                    //no opt for arrow keys
                }
            },
            Event::Tick => {
                self.advance();
            }
        }
        //dont break from here
        Some(())
    }

    pub fn handle_nav(&mut self, event: Event<Key>, aha: &Aha) -> Option<()> {
        match event {
            Event::Input(input) => match input {
                Key::Char('q') => None,

                Key::Char('s') => {
                    self.debug_txt = format!("search");
                    self.popup = Popup::Search;
                    Some(())
                }
                Key::Char('c') => {
                    self.debug_txt = format!("create");
                    self.popup = Popup::Text;
                    Some(())
                }
                Key::Left | Key::Char('h') => {
                    self.debug_txt = format!("back");
                    if self.active_layer == 0 {}
                    if self.active_layer == 1 {
                        self.releases.unselect();
                        self.active_layer = 0;
                    }

                    if self.active_layer == 2 {
                        self.features.unselect();
                        self.active_layer = 1;
                    }

                    if self.active_layer == 3 {
                        self.active_layer = 2;
                    }

                    Some(())
                }
                Key::Right | Key::Char('l') | Key::Char('\n') => {
                    self.debug_txt = format!("over");
                    if self.active_layer == 2 {
                        if self.features.state.selected().is_some() {
                            self.active_layer = 3;
                            self.format_selected_feature();
                        };
                    }
                    if self.active_layer == 1 {
                        match self.releases.state.selected() {
                            Some(i) => {
                                self.active_layer = 2;
                                let project = self.releases.items[i].clone();
                                let releases =
                                    aha.features(project.1["id"].as_str().unwrap().to_string());

                                self.features = StatefulList::with_items(
                                    releases
                                        .iter()
                                        .map(|project| {
                                            (
                                                format!(
                                                    "{} - {}",
                                                    project["name"],
                                                    project["workflow_status"]["name"]
                                                ),
                                                project.clone(),
                                            )
                                        })
                                        .collect(),
                                );
                            }
                            None => {}
                        };
                    }
                    if self.active_layer == 0 {
                        match self.items.state.selected() {
                            Some(i) => {
                                self.active_layer = 1;
                                let project = self.items.items[i].clone();
                                let releases =
                                    aha.releases(project.1["id"].as_str().unwrap().to_string());

                                self.releases = StatefulList::with_items(
                                    releases
                                        .iter()
                                        .map(|project| {
                                            (project["name"].to_string(), project.clone())
                                        })
                                        .collect(),
                                );
                            }
                            None => {}
                        };
                    }

                    Some(())
                }
                Key::Down | Key::Char('j') => {
                    self.debug_txt = format!("down");
                    if self.active_layer == 0 {
                        self.items.next();
                    }
                    if self.active_layer == 1 {
                        self.releases.next();
                    }
                    if self.active_layer == 2 {
                        self.features.next();
                    }

                    if self.active_layer == 3 {
                        self.features.next();
                        self.format_selected_feature();
                    }

                    Some(())
                }
                Key::Up | Key::Char('k') => {
                    self.debug_txt = format!("up");
                    if self.active_layer == 0 {
                        self.items.previous();
                    }
                    if self.active_layer == 1 {
                        self.releases.previous();
                    }
                    if self.active_layer == 2 {
                        self.features.previous();
                    }

                    if self.active_layer == 3 {
                        self.features.previous();
                        self.format_selected_feature();
                    }

                    Some(())
                }
                _ => {
                    self.debug_txt = format!("{:?}", input);
                    Some(())
                }
            },
            Event::Tick => {
                self.advance();
                Some(())
            }
        }
    }
}
