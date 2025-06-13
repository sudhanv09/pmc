#![allow(warnings)]

use crate::indexer;
use crate::indexer::{Episode, Library, Season, Tv};
use chrono::{DateTime, Local};
use colored::Colorize;
use dialoguer::Input;
use dialoguer::theme::ColorfulTheme;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MediaType {
    Movie,
    Show,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchEntry {
    pub id: String,
    pub media_id: String,
    pub media_type: MediaType,
    pub progress: i16,
    pub complete: bool,
    pub watched_at: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub media_id: Option<String>,
    pub file_path: Option<PathBuf>,
    pub media_type: Option<MediaType>,
    pub percent_pos: Option<f64>,
    pub is_playing: bool,
    pub should_stop: bool, // For graceful shutdown
}

#[derive(Debug)]
pub enum PlaybackEvent {
    Started,
    Stopped,
    Paused,
    Resumed,
    Position(f64),
    Completed,
    Error(String),
}

pub struct ShowPlaybackSession {
    pub episodes: Vec<Episode>,
    pub current_index: usize,
}

pub type SharedState = Arc<Mutex<PlaybackState>>;

impl PlaybackState {
    pub fn new() -> Self {
        Self {
            media_id: None,
            file_path: None,
            media_type: None,
            percent_pos: None,
            is_playing: false,
            should_stop: false,
        }
    }

    pub fn init(&mut self, media_id: String, file_path: PathBuf, media_type: MediaType) {
        self.media_id = Some(media_id);
        self.file_path = Some(file_path);
        self.media_type = Some(media_type);
        self.is_playing = true;
        self.should_stop = false;
    }

    pub fn update_position(&mut self, percent_pos: f64) {
        self.percent_pos = Some(percent_pos);
    }

    pub fn stop_playback(&mut self) {
        self.is_playing = false;
    }

    pub fn should_save_progress(&self) -> bool {
        self.media_id.is_some() && self.percent_pos.is_some()
    }
    
    pub fn mark_completed(&mut self) {
        self.percent_pos = Some(100.0);
        self.is_playing = false;
    }

    // Helpers
    pub fn is_completed(&self) -> bool {
        self.percent_pos.map_or(false, |pos| pos > 96.0)
    }

    pub fn completion_percentage(&self) -> f64 {
        self.percent_pos.unwrap_or(0.0)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaLibrary {
    pub movie_dir: String,
    pub tv_dir: String,
    pub indexed_at: DateTime<Local>,
    pub library: Library,

    #[serde(skip)]
    pub show_map: HashMap<String, Tv>,
    #[serde(skip)]
    pub episode_map: HashMap<String, (Tv, Season, Episode)>,
}

impl MediaLibrary {
    pub fn new(movie_dir: String, tv_dir: String, library: Library) -> Self {
        let mut show_map = HashMap::new();
        let mut episode_map = HashMap::new();

        for show in &library.shows {
            show_map.insert(show.id.clone(), show.clone());
            for season in &show.seasons {
                for episode in &season.episodes {
                    episode_map.insert(
                        episode.id.clone(),
                        (show.clone(), season.clone(), episode.clone()),
                    );
                }
            }
        }

        Self {
            movie_dir,
            tv_dir,
            indexed_at: Local::now(),
            library,
            show_map,
            episode_map,
        }
    }

    pub fn rebuild_maps(&mut self) {
        self.show_map.clear();
        self.episode_map.clear();

        for show in &self.library.shows {
            self.show_map.insert(show.id.clone(), show.clone());
            for season in &show.seasons {
                for episode in &season.episodes {
                    self.episode_map.insert(
                        episode.id.clone(),
                        (show.clone(), season.clone(), episode.clone()),
                    );
                }
            }
        }
    }
}

pub async fn load_or_configure_library() -> tokio::io::Result<MediaLibrary> {
    let index_path = "./index.json";

    if Path::new(index_path).exists() {
        let data = fs::read_to_string(index_path)?;
        let mut library: MediaLibrary = serde_json::from_str(&data)?;
        library.rebuild_maps(); // 🔧 Restore runtime-only maps
        Ok(library)
    } else {
        println!("{}", "Welcome to pmc!".blue());

        let movie_dir: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter movie directory")
            .interact_text()
            .unwrap();

        let tv_dir: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter tv directory")
            .interact_text()
            .unwrap();

        let library = indexer::index(movie_dir.clone(), tv_dir.clone());
        let media_library = MediaLibrary::new(movie_dir, tv_dir, library);

        let serialized = serde_json::to_string_pretty(&media_library)?;
        fs::write(index_path, serialized)?;

        Ok(media_library)
    }
}
