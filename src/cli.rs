#![allow(warnings)]

use crate::db;
use crate::indexer::Library;
use crate::library::{MediaType, WatchEntry, flatten_show};
use crate::mpv::Player;
use chrono::Local;
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use nanoid::nanoid;

#[derive(Parser)]
#[command(name = "pmc")]
#[command(about = "simple CLI tool to manage your media", long_about = None)]
pub struct App {
    #[arg(
        short,
        long,
        help = "Path to the MPV IPC socket",
        default_value = "/tmp/mpvipc"
    )]
    pub mpv_socket: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List,
    Recent,
    Play,
    Resume,
}

pub async fn handle_list_command(index: &Library) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Movies".green());
    println!("------");
    for item in &index.movies {
        println!("{}", &item.name);
    }
    println!("\n");

    println!("{}", "Shows".green());
    println!("------");
    for item in &index.shows {
        println!("{}", &item.name);
    }
    Ok(())
}

pub async fn handle_recent_command(index: &Library) -> Result<(), Box<dyn std::error::Error>> {
    let db = db::Db::get();
    let recent = db.get_recent_watches(5).await?;

    if recent.is_empty() {
        println!("{}", "No recent watches found.".yellow());
        return Ok(());
    }

    println!("{}", "Recent watches found.".green());
    for item in &recent {
        match item.media_type {
            MediaType::Movie => {
                if let Some(movie) = index.movies.iter().find(|m| m.id == item.media_id) {
                    println!(
                        "{} | {} | Progress: {}% | Completed: {}",
                        movie.name.cyan(),
                        item.watched_at.format("%Y-%m-%d %H:%M"),
                        item.progress,
                        if item.complete { "Yes" } else { "No" }
                    );
                } else {
                    println!("{} (Movie ID not found)", item.media_id.red());
                }
            }
            MediaType::Show => {
                if let Some(tv) = index.shows.iter().find(|m| m.id == item.media_id) {
                    println!(
                        "{} | {} | Progress: {}% | Completed: {}",
                        tv.name.cyan(),
                        item.watched_at.format("%Y-%m-%d %H:%M"),
                        item.progress,
                        if item.complete { "Yes" } else { "No" }
                    );
                } else {
                    println!("{} (Show ID not found)", item.media_id.red());
                }
            }
        }
    }
    Ok(())
}

pub async fn handle_resume_command(
    index: &Library,
    player: &Player,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub async fn handle_play_command(
    index: &Library,
    mut player: Player,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = db::Db::get();
    let media_select = &["Movies", "Tv"];
    let selections = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick your food")
        .items(&media_select[..])
        .interact()
        .unwrap();

    println!("Select one of the {}", media_select[0]);

    match selections {
        0 => {
            if index.movies.is_empty() {
                println!("No movies found");
                return Ok(());
            }

            let movie_titles = &index
                .movies
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>();
            let choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Pick a movie to watch")
                .items(&movie_titles[..])
                .interact()
                .unwrap();

            let movie = &index.movies[choice];
            println!("Now playing movie: {}", movie.name);

            player
                .play_file(movie.path.clone())
                .await
                .expect("Unable to play file");

            let (progress, complete) = player.monitor_playback().await?;
            let entry = WatchEntry {
                id: nanoid!(),
                media_id: movie.id.clone(),
                media_type: MediaType::Movie,
                progress,
                complete,
                watched_at: Local::now(),
            };

            db.save_state(&entry).await.expect("Unable to save state");
        }
        1 => {
            if index.shows.is_empty() {
                println!("No TV shows found.");
                return Ok(());
            }

            let show_titles: Vec<&str> = index.shows.iter().map(|s| s.name.as_str()).collect();

            let show_choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Pick a show")
                .items(&show_titles)
                .interact()
                .unwrap();

            let show = &index.shows[show_choice];
            println!("Now playing first episode of show: {}", show.name);

            let show_history = db.get_media_history(show.id.clone()).await?;
            let episodes = flatten_show(&show);

            let last_watched = show_history
                .iter()
                .filter(|entry| entry.media_type == MediaType::Show)
                .max_by_key(|entry| entry.watched_at);

            let to_play = match last_watched {
                None => {
                    println!("No history found. Starting from the beginning.");
                    episodes.first()
                }
                Some(entry) => {
                    let ep_index = episodes.iter().position(|ep| ep.id == entry.media_id);

                    match (ep_index, entry.complete) {
                        (Some(idx), false) => {
                            println!("Resume unfinished episode: {}", episodes[idx].name);
                            Some(&episodes[idx])
                        }
                        (Some(idx), true) => {
                            if let Some(next_ep) = episodes.get(idx + 1) {
                                println!("Playing next episode: {}", next_ep.name);
                                Some(next_ep)
                            } else {
                                println!("You've finished all available episodes.");
                                None
                            }
                        }
                        _ => {
                            println!("Corrupted history. Starting from beginning.");
                            episodes.first()
                        }
                    }
                }
            };

            if let Some(episode) = to_play {
                println!("Now playing: {}", episode.name);
                player.play_file(episode.path.clone()).await?;
                let (progress, complete) = player.monitor_playback().await?;
                let entry = WatchEntry {
                    id: nanoid!(),
                    media_id: show.id.clone(),
                    media_type: MediaType::Show,
                    progress,
                    complete,
                    watched_at: Local::now(),
                };

                db.save_state(&entry).await.expect("Unable to save state");
            } else {
                println!("Nothing to play.");
            }
        }
        _ => {}
    };
    Ok(())
}
