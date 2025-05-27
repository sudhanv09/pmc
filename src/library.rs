use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum MediaType {
    Movie,
    ShowEpisode,
}

pub struct WatchEntry {
    pub media_type: MediaType,
    pub media_id: String,
    pub complete: bool,
    pub watched_at: DateTime<Utc>,
    pub percent_watched: f32,
}

pub struct ShowHistory {
    pub episodes: HashMap<String, WatchEntry>,
}

pub struct WatchHistory {
    pub recent: VecDeque<WatchEntry>,
    pub movie_history: HashMap<String, WatchEntry>,
    pub series_history: HashMap<String, WatchEntry>,
}