#![allow(warnings)]

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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