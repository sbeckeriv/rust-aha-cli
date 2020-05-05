use super::util::StatefulList;

use serde_json::Value;
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, Paragraph, Text},
    Terminal,
};
pub struct App<'a> {
    pub items: StatefulList<(String, Value)>,
    pub releases: StatefulList<(String, Value)>,
    pub features: StatefulList<(String, Value)>,
    pub feature_text: Vec<String>,
    pub active_layer: i8,
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
            feature_text: vec!["Loading..".to_string()],
            active_layer: 0,
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
}
