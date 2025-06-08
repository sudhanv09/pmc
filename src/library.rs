#![allow(warnings)]

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use crate::indexer::{Episode, Tv};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MediaType {
    Movie,
    Show
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

pub fn flatten_show(show: &Tv) -> Vec<Episode> {
    let mut entries = Vec::new();
    
    for item in show.seasons.iter() {
        for episode in item.episodes.iter() {
            entries.push(episode.clone());
        }
    }
    
    entries
}