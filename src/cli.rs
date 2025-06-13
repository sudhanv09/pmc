#![allow(warnings)]

use crate::db;
use crate::db::Db;
use crate::indexer::{Episode, Tv};
use crate::library::{MediaLibrary, MediaType, PlaybackEvent, PlaybackState, SharedState, WatchEntry};
use crate::mpv::{Player, spawn_mpv};
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

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

pub async fn handle_list_command(index: &MediaLibrary) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Movies".green());
    println!("------");
    for item in &index.library.movies {
        println!("{}", &item.name);
    }
    println!("\n");

    println!("{}", "Shows".green());
    println!("------");
    for item in &index.library.shows {
        println!("{}", &item.name);
    }
    Ok(())
}

pub async fn handle_recent_command(index: &MediaLibrary) -> Result<(), Box<dyn std::error::Error>> {
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
                if let Some(movie) = index.library.movies.iter().find(|m| m.id == item.media_id) {
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
                if let Some((tv, season, episode)) = index.episode_map.get(&item.media_id) {
                    println!(
                        "{} (S{} E{}) \n\t {} \t Progress: {}% \t Completed: {}",
                        tv.name.cyan(),
                        season.number,
                        episode.name,
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

pub async fn handle_resume_command(index: &MediaLibrary) -> Result<(), Box<dyn std::error::Error>> {
    let db = db::Db::get();
    let recent = db.get_recent_watches(10).await?;

    // Filter for incomplete items
    let mut incomplete: Vec<_> = recent.into_iter().filter(|entry| !entry.complete).collect();

    if incomplete.is_empty() {
        println!("No incomplete watches to resume.");
        return Ok(());
    }
    
    let to_resume = &incomplete[0];
    let state: SharedState = Arc::new(Mutex::new(PlaybackState::new()));
    let socket_path = "/tmp/pmc-resume.sock";

    // Spawn mpv
    spawn_mpv(socket_path)?;
    sleep(Duration::from_millis(300)).await; // wait for socket

    let mut player = Player::init(socket_path).await?;
    let (tx, mut rx) = mpsc::channel::<PlaybackEvent>(16);
    
    match to_resume.media_type {
        MediaType::Movie => {
            if let Some(movie) = index
                .library
                .movies
                .iter()
                .find(|m| m.id == to_resume.media_id)
            {
                {
                    let mut guard = state.lock().unwrap();
                    guard.start_playback(movie.id.clone(), movie.path.clone(), MediaType::Movie);
                }

                println!("Resuming movie: {}", movie.name);
                player.play_file(movie.path.clone()).await?;
                let monitor_state = state.clone();
                tokio::spawn(async move {
                    let _ = player.start_monitoring(monitor_state, tx);
                });
            }
        }
        MediaType::Show => {
            if let Some((_, _, episode)) = index.episode_map.get(&to_resume.media_id) {
                {
                    let mut guard = state.lock().unwrap();
                    guard.start_playback(episode.id.clone(), episode.path.clone(), MediaType::Show);
                }

                println!("Resuming episode: {}", episode.name);
                player.play_file(episode.path.clone()).await?;
                let monitor_state = state.clone();
                tokio::spawn(async move {
                    let _ = player.start_monitoring(monitor_state, tx);
                });
            }
        }
    }

    while let Some(event) = rx.recv().await {
        match event {
            PlaybackEvent::Paused => println!("{}", "⏸ Paused".yellow()),
            PlaybackEvent::Resumed => println!("{}", "▶️ Resumed".green()),
            PlaybackEvent::Started => println!("{}", "▶️ Playback started".cyan()),
            PlaybackEvent::Position(p) => println!("⏱ {:.1}% played", p),
            PlaybackEvent::Completed => {
                println!("{}", "✅ Playback complete".blue());
                break;
            }
            PlaybackEvent::Stopped => {
                println!("{}", "🛑 Playback stopped".red());
                break;
            }
            PlaybackEvent::Error(e) => {
                eprintln!("❌ Playback error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

pub async fn handle_play_command(index: &MediaLibrary) -> Result<(), Box<dyn std::error::Error>> {
    let db = db::Db::get();

    let state: SharedState = Arc::new(Mutex::new(PlaybackState::new()));

    let media_select = &["Movies", "Tv"];
    let selections = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What do you want to watch?")
        .items(&media_select[..])
        .interact()
        .unwrap();

    match selections {
        0 => handle_movie_playback(index, &db, state).await?,
        1 => handle_show_playback(index, &db, state).await?,
        _ => {}
    };
    Ok(())
}

async fn handle_movie_playback(
    index: &MediaLibrary,
    db: &db::Db,
    state: SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    if index.library.movies.is_empty() {
        println!("No movies found");
        return Ok(());
    }

    let movie_titles: Vec<&str> = index
        .library
        .movies
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a movie to watch")
        .items(&movie_titles)
        .interact()
        .unwrap();

    let movie = &index.library.movies[choice];

    {
        let mut state_guard = state.lock().unwrap();
        state_guard.start_playback(movie.id.clone(), movie.path.clone(), MediaType::Movie);
    }

    println!("{}", "Spawning mpv".yellow());
    let socket_path = "/tmp/pmc-mpv.sock";
    spawn_mpv(socket_path)?;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("{}", "Mpv running".green());
    println!("Now playing movie: {}", movie.name);

    let mut mpv = Player::init(socket_path).await?;
    mpv.play_file(movie.path.clone()).await?;

    // Spawn MPV monitor task
    let (tx, mut rx) = mpsc::channel::<PlaybackEvent>(16);
    let monitor_state = state.clone();
    tokio::spawn(async move {
        let _ = mpv.start_monitoring(monitor_state, tx);
    });

    // Listen for events
    while let Some(event) = rx.recv().await {
        match event {
            PlaybackEvent::Paused => println!("{}", "Paused".yellow()),
            PlaybackEvent::Resumed => println!("{}", "Resumed".green()),
            PlaybackEvent::Position(pos) => {
                println!("Progress: {:.1}%", pos);
            }
            PlaybackEvent::Completed => {
                println!("{}", "Playback complete!".blue());
                break;
            }
            PlaybackEvent::Stopped => {
                println!("{}", "Playback stopped.".red());
                break;
            }
            PlaybackEvent::Error(e) => {
                eprintln!("MPV error: {}", e);
                break;
            }
            PlaybackEvent::Started => {
                println!("{}", "Playback started.".green());
            }
        }
    }
    if let Err(e) = db.save_playback_progress(state.clone()).await {
        println!("Failed to save progress: {}", e);
    }

    Ok(())
}

pub fn flatten_show(show: &Tv) -> Vec<Episode> {
    let mut entries = Vec::new();

    for item in show.seasons.iter() {
        for episode in item.episodes.iter() {
            entries.push(episode.clone());
        }
    }

    entries
}

async fn handle_show_playback(
    index: &MediaLibrary,
    db: &Db,
    state: SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    if index.library.shows.is_empty() {
        println!("No TV shows found.");
        return Ok(());
    }

    let show_titles: Vec<&str> = index
        .library
        .shows
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a show")
        .items(&show_titles)
        .interact()
        .unwrap();

    let show = &index.library.shows[choice];
    let episodes = flatten_show(show);
    let history = db.get_media_history(show.id.clone()).await?;

    let to_play = select_episode_to_play(&episodes, &history);

    if let Some(episode) = to_play {
        let mut state_guard = state.lock().unwrap();
        state_guard.start_playback(episode.id.clone(), episode.path.clone(), MediaType::Show);
        drop(state_guard);

        println!("{}", "Spawning mpv".yellow());
        let socket_path = "/tmp/pmc-mpv.sock";
        spawn_mpv(socket_path)?;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        println!("{}", "Mpv running".green());
        println!("Now playing: {}", episode.name);

        let mut mpv = Player::init(socket_path).await?;
        mpv.play_file(episode.path.clone()).await?;
        
        // Spawn MPV monitor task
        let (tx, mut rx) = mpsc::channel::<PlaybackEvent>(16);
        let monitor_state = state.clone();
        tokio::spawn(async move {
            let _ = mpv.start_monitoring(monitor_state, tx);
        });

        while let Some(event) = rx.recv().await {
            match event {
                PlaybackEvent::Paused => println!("{}", "Paused".yellow()),
                PlaybackEvent::Resumed => println!("{}", "Resumed".green()),
                PlaybackEvent::Position(pos) => {
                    println!("Progress: {:.1}%", pos);
                }
                PlaybackEvent::Completed => {
                    println!("{}", "Playback complete!".blue());
                    break;
                }
                PlaybackEvent::Stopped => {
                    println!("{}", "Playback stopped.".red());
                    break;
                }
                PlaybackEvent::Error(e) => {
                    eprintln!("MPV error: {}", e);
                    break;
                }
                PlaybackEvent::Started => {
                    println!("{}", "Playback started.".green());
                }
            }
        }

        if let Err(e) = db.save_playback_progress(state.clone()).await {
            println!("Failed to save progress: {}", e);
        }

        Ok(())
    } else {
        println!("Nothing to play.");
        Ok(())
    }
}

fn select_episode_to_play<'a>(
    episodes: &'a [Episode],
    history: &[WatchEntry],
) -> Option<&'a Episode> {
    let last = history
        .iter()
        .filter(|entry| entry.media_type == MediaType::Show)
        .max_by_key(|entry| entry.watched_at);

    match last {
        None => episodes.first(),
        Some(entry) => {
            let idx = episodes.iter().position(|ep| ep.id == entry.media_id);
            match (idx, entry.complete) {
                (Some(i), false) => Some(&episodes[i]),
                (Some(i), true) => episodes.get(i + 1),
                _ => episodes.first(),
            }
        }
    }
}
