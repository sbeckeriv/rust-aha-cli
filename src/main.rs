mod app;
#[allow(dead_code)]
mod util;

use app::App;
use std::{error::Error, io};
use termion::{
    event::Key, event::MouseEvent, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen,
};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, Paragraph, Text},
    Terminal,
};
use util::{
    event::{Event, Events},
    StatefulList,
};

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
extern crate structopt;
#[macro_use]
extern crate prettytable;
extern crate notify_rust;
extern crate regex;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use structopt::StructOpt;
mod aha;
mod github;

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
    #[structopt(short = "g", long = "generate")]
    generate: bool,
    #[structopt(short = "p", long = "prs")]
    pr_status: bool,
    #[structopt(long = "closed")]
    closed: bool,
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
    github_api_token: String,
    aha_domain: String,
    aha_token: String,
    workflow_repo: String,
    workflow_login: String,
    workflow_email: String,
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
                Err(why) => panic!("couldn't open {}: {}", display, why.description()),
                Ok(file) => file,
            };

            // Read the file contents into a string, returns `io::Result<usize>`
            let mut s = String::new();
            match file.read_to_string(&mut s) {
                Err(why) => panic!("couldn't read {}: {}", display, why.description()),
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
    let aha = aha::Aha::new(
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

    let events = Events::new();

    // App
    let mut app = App::new();
    let aha_projects = aha.projects();
    app.items = StatefulList::with_items(
        aha_projects
            .iter()
            .map(|project| (project["name"].to_string(), project.clone()))
            .collect(),
    );
    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(15), Constraint::Percentage(85)].as_ref())
                .split(f.size());
            let mut project_size = 10;
            let mut release_size = 90;
            if app.active_layer == 0 {
                project_size = 90;
                release_size = 10;
            }
            let mut release_chunks = Layout::default()
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
                .style(style)
                .highlight_style(style.fg(Color::LightGreen).modifier(Modifier::BOLD))
                .highlight_symbol(">");
            f.render_stateful_widget(items, release_chunks[0], &mut app.items.state);

            let releases_items = app.releases.items.iter().map(|i| Text::raw(i.0.clone()));
            let releases_items = List::new(releases_items)
                .block(Block::default().borders(Borders::ALL).title("Releases"))
                .style(style)
                .highlight_style(style.fg(Color::LightGreen).modifier(Modifier::BOLD))
                .highlight_symbol(">");
            f.render_stateful_widget(releases_items, release_chunks[1], &mut app.releases.state);

            let feature_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(20),
                        Constraint::Percentage(75),
                        Constraint::Percentage(5),
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            let feature_items = app.features.items.iter().map(|i| Text::raw(i.0.clone()));
            let feature_items = List::new(feature_items)
                .block(Block::default().borders(Borders::ALL).title("Features"))
                .style(style)
                .highlight_style(style.fg(Color::LightGreen).modifier(Modifier::BOLD))
                .highlight_symbol(">");
            f.render_stateful_widget(feature_items, feature_chunks[0], &mut app.features.state);
            let paragraph = Paragraph::new(app.feature_text_formatted.iter())
                .block(Block::default().title("Feature").borders(Borders::ALL))
                .wrap(true);
            f.render_widget(paragraph, feature_chunks[1]);
            let events = app
                .events
                .iter()
                .map(|&(evt, level)| Text::raw(format!("{}", app.debug_txt)));
            let events_list = List::new(events)
                .block(Block::default().borders(Borders::ALL).title("dbg"))
                .start_corner(Corner::BottomLeft);
            f.render_widget(events_list, feature_chunks[2]);
        })?;

        match events.next()? {
            Event::Input(input) => match input {
                Key::Char('q') => {
                    break;
                }
                Key::Left | Key::Char('h') => {
                    if app.active_layer == 0 {}
                    if app.active_layer == 1 {
                        app.releases.unselect();
                        app.active_layer = 0;
                    }

                    if app.active_layer == 2 {
                        app.features.unselect();
                        app.active_layer = 1;
                    }

                    if app.active_layer == 3 {
                        app.active_layer = 2;
                    }
                }
                Key::Right | Key::Char('l') => {
                    if app.active_layer == 2 {
                        if app.features.state.selected().is_some() {
                            app.active_layer = 3;
                            app.format_selected_feature();
                        };
                    }
                    if app.active_layer == 1 {
                        match app.releases.state.selected() {
                            Some(i) => {
                                app.active_layer = 2;
                                let project = app.releases.items[i].clone();
                                let releases =
                                    aha.features(project.1["id"].as_str().unwrap().to_string());

                                app.features = StatefulList::with_items(
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
                    if app.active_layer == 0 {
                        match app.items.state.selected() {
                            Some(i) => {
                                app.active_layer = 1;
                                let project = app.items.items[i].clone();
                                let releases =
                                    aha.releases(project.1["id"].as_str().unwrap().to_string());

                                app.releases = StatefulList::with_items(
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
                }
                Key::Down | Key::Char('j') => {
                    if app.active_layer == 0 {
                        app.items.next();
                    }
                    if app.active_layer == 1 {
                        app.releases.next();
                    }
                    if app.active_layer == 2 {
                        app.features.next();
                    }

                    if app.active_layer == 3 {
                        app.features.next();
                        app.format_selected_feature();
                    }
                }
                Key::Up | Key::Char('k') => {
                    if app.active_layer == 0 {
                        app.items.previous();
                    }
                    if app.active_layer == 1 {
                        app.releases.previous();
                    }
                    if app.active_layer == 2 {
                        app.features.previous();
                    }

                    if app.active_layer == 3 {
                        app.features.previous();
                        app.format_selected_feature();
                    }
                }
                _ => {}
            },
            Event::Tick => {
                app.advance();
            }
        }
    }

    Ok(())
}
