use std::path::PathBuf;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

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
    Position(f64, String),
    Completed,
    Exited,
    RequestNext,
    RequestPrev,
    RequestQuit,
    Error(String),
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

    #[allow(unused)]
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