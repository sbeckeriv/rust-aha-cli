mod app;
#[allow(dead_code)]
mod util;

extern crate dirs;
extern crate dotenv;
extern crate envy;
extern crate scarlet;
extern crate termion;
#[macro_use]
extern crate failure;
extern crate env_logger;
extern crate log;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate regex;
extern crate structopt;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use structopt::StructOpt;
mod aha;

use aha::Aha;
use app::{App, Popup};
use std::{error::Error, io};
use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, Paragraph, Text},
    Terminal,
};
use util::{event::Events, StatefulList};

#[derive(StructOpt, Debug)]
pub struct Opt {
    #[structopt(short = "r", long = "repo", name = "repo")]
    repo: Option<String>,
    #[structopt(short = "d", long = "dryrun")]
    dry_run: bool,
    #[structopt(short = "s", long = "silent")]
    silent: bool,
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
    #[structopt(short = "c", long = "config")]
    config_file: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Config {
    aha: Option<AhaConfig>,
    global_integer: Option<u64>,
    repos: Option<Vec<RepoConfig>>,
}

#[derive(Debug, Deserialize)]
struct RepoConfig {
    name: String,
    username: String,
    labels: Option<HashMap<String, String>>,
}
#[derive(Debug, Deserialize)]
struct AhaConfig {
    domain: String,
    email: String,
}

#[derive(Deserialize, Debug)]
struct Env {
    aha_domain: String,
    aha_token: String,
    workflow_email: String,
}

use tui::layout::Rect;
use tui::widgets::Clear;

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn load_config() -> Result<(Env, Opt), Box<dyn Error>> {
    //copied config
    let opt = Opt::from_args();
    if opt.verbose {
        println!("{:?}", opt);
    }
    let home_dir = dirs::home_dir().expect("Could not find home path");

    let path_name = match &opt.config_file {
        Some(path) => path.clone(),
        None => format!("{}/.aha_workflow", home_dir.display()),
    };

    if opt.verbose {
        println!("{:?}", path_name);
    }
    let config_path = fs::canonicalize(&path_name);
    let config_info: Option<Config> = match config_path {
        Ok(path) => {
            if opt.verbose {
                println!("found {:?}", path_name);
            }
            let display = path.display();
            let mut file = match File::open(&path) {
                Err(why) => panic!("couldn't open {}: {}", display, why.to_string()),
                Ok(file) => file,
            };

            // Read the file contents into a string, returns `io::Result<usize>`
            let mut s = String::new();
            match file.read_to_string(&mut s) {
                Err(why) => panic!("couldn't read {}: {}", display, why.to_string()),
                Ok(_) => (),
            }
            Some(toml::from_str(&s)?)
        }
        Err(e) => {
            if !opt.silent {
                println!("did not find {:?}, {}", path_name, e);
            }
            None
        }
    };

    //dotenv::dotenv().ok();
    let my_path = format!("{}/.env", home_dir.display());
    dotenv::from_path(my_path).ok();
    env_logger::init();

    let mut config: Env = envy::from_env()?;

    match config_info.as_ref() {
        Some(c) => match c.aha.as_ref() {
            Some(a) => {
                config.aha_domain = a.domain.clone();
                config.workflow_email = a.email.clone();
            }
            _ => (),
        },
        _ => (),
    }

    if opt.verbose {
        println!("config updated");
    }

    Ok((config, opt))
}

fn main() -> Result<(), Box<dyn Error>> {
    let (config, opt) = load_config().unwrap();
    let aha = Aha::new(
        config.aha_domain,
        config.aha_token,
        config.workflow_email,
        &opt,
    );

    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    //terminal.hide_cursor()?;

    let mut events = Events::new();

    // App
    let mut app = App::new();
    let aha_projects = aha.projects();
    app.items = StatefulList::with_items(
        aha_projects
            .iter()
            .map(|project| (project["name"].to_string(), project.clone()))
            .collect(),
    );

    let home_dir = dirs::home_dir().expect("Could not find home path");

    let path_name = format!("{}/.aha_cli_cache", home_dir.display());
    match File::open(&path_name) {
        Err(why) => {
            if opt.verbose {
                println!("couldn't open {}: {}", path_name, why.to_string());
            }
        }
        Ok(mut file) => {
            let mut s = String::new();
            match file.read_to_string(&mut s) {
                Err(why) => panic!("couldn't read {}: {}", path_name, why),
                Ok(_) => (),
            }
            app.load_history(s, &aha);
        }
    };
    loop {
        terminal.draw(|mut f| {
            app.help_text();
            let mut menu = 30;
            let mut main = 70;
            if app.active_layer >= 2 {
                menu = 0;
                main = 100;
            }
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(menu), Constraint::Percentage(main)].as_ref())
                .split(f.size());
            let mut project_size = 10;
            let mut release_size = 90;
            if app.active_layer == 0 {
                project_size = 90;
                release_size = 10;
            }
            let release_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(project_size),
                        Constraint::Percentage(release_size),
                    ]
                    .as_ref(),
                )
                .split(chunks[0]);

            let style = Style::default().fg(Color::Black).bg(Color::White);

            let items = app.items.items.iter().map(|i| Text::raw(i.0.clone()));
            let items = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Projects"))
                .highlight_style(style.fg(Color::Black).modifier(Modifier::BOLD))
                .highlight_symbol(">");
            f.render_stateful_widget(items, release_chunks[0], &mut app.items.state);

            let releases_items = app.releases.items.iter().map(|i| Text::raw(i.0.clone()));
            let releases_items = List::new(releases_items)
                .block(Block::default().borders(Borders::ALL).title("Releases"))
                .highlight_style(style.fg(Color::Black).modifier(Modifier::BOLD))
                .highlight_symbol(">");
            f.render_stateful_widget(releases_items, release_chunks[1], &mut app.releases.state);
            let mut feature_list = 10;
            let mut feature_show = 85;
            if app.active_layer == 2 {
                feature_list = 40;
                feature_show = 55;
            }
            let feature_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(feature_list),
                        Constraint::Percentage(feature_show),
                        Constraint::Percentage(5),
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            let feature_items = app.features.items.iter().map(|i| Text::raw(i.0.clone()));
            let feature_items = List::new(feature_items)
                .block(Block::default().borders(Borders::ALL).title("Features"))
                .highlight_style(style.fg(Color::Black).modifier(Modifier::BOLD))
                .highlight_symbol(">");
            f.render_stateful_widget(feature_items, feature_chunks[0], &mut app.features.state);
            let paragraph = Paragraph::new(app.feature_text_formatted.iter())
                .block(Block::default().title("Feature").borders(Borders::ALL))
                .wrap(true);
            f.render_widget(paragraph, feature_chunks[1]);
            let events_list = app
                .events
                .iter()
                .map(|&(_, _)| Text::raw(format!("{}", app.debug_txt)));
            let events_list = List::new(events_list)
                .block(Block::default().borders(Borders::ALL).title("dbg"))
                .start_corner(Corner::BottomLeft);
            f.render_widget(events_list, feature_chunks[2]);
            match app.popup {
                Popup::Text => {
                    let block = Block::default()
                        .title(&app.text_box_title)
                        .borders(Borders::ALL);
                    let text = Text::raw(app.text_box.clone());
                    let text_vec = vec![text];
                    let create_paragraph = Paragraph::new(text_vec.iter()).block(block).wrap(true);
                    let size = f.size();
                    let area = centered_rect(60, 20, size);
                    f.render_widget(Clear, area); //this clears out the background
                    f.render_widget(create_paragraph, area);
                }
                Popup::Search => {
                    let block = Block::default().title("Search").borders(Borders::ALL);
                    let text = Text::raw(app.text_box.clone());
                    let text_vec = vec![text];
                    let create_paragraph = Paragraph::new(text_vec.iter()).block(block).wrap(true);
                    let size = f.size();
                    let area = centered_rect(60, 50, size);
                    f.render_widget(Clear, area); //this clears out the background
                    f.render_widget(create_paragraph, area);
                }
                _ => {}
            }
        })?;

        if let Ok(event) = events.next() {
            let result = if app.popup == Popup::Text && app.releases.state.selected().is_some() {
                let x = app.handle_create_popup(event, &aha);
                events.disable_exit_key();
                x
            } else if app.popup == Popup::Search {
                let x = app.handle_search_popup(event, &aha);
                events.disable_exit_key();
                x
            } else {
                events.enable_exit_key();
                app.handle_nav(event, &aha)
            };
            if result.is_none() {
                break;
            }
        }
    }

    Ok(())
}
