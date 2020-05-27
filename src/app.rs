use super::util::StatefulList;
use scarlet::color;
use scarlet::color::RGBColor;

use serde_json::Value;
use termion::color::Rgb;
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, Paragraph, Text},
    Terminal,
};

use super::aha::FeatureCreate;
pub struct App<'a> {
    pub items: StatefulList<(String, Value)>,
    pub releases: StatefulList<(String, Value)>,
    pub features: StatefulList<(String, Value)>,
    pub feature_text: Vec<String>,
    pub debug_txt: String,
    pub feature_text_formatted: Vec<Text<'a>>,
    pub active_layer: i8,
    pub show_text_box: bool,
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
        App {
            items: StatefulList::with_items(vec![]),
            releases: StatefulList::with_items(vec![]),
            features: StatefulList::with_items(vec![]),
            feature_text: vec!["".to_string()],
            feature_text_formatted: vec![Text::raw("")],
            debug_txt: "".to_string(),
            active_layer: 0,
            show_text_box: false,
            new_feature: FeatureCreate::new(),
            text_box: "".to_string(),
            text_box_title: "Feature Name".to_string(),
            events: vec![("Event1", "INFO")],
            info_style: Style::default().fg(Color::White),
            warning_style: Style::default().fg(Color::Yellow),
            error_style: Style::default().fg(Color::Magenta),
            critical_style: Style::default().fg(Color::Red),
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
}
