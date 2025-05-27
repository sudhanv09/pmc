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
    pub media_id: String,
    pub media_type: MediaType,
    pub progress: i16,
    pub complete: bool, 
    pub watched_at: DateTime<Local>,
}

pub fn get_recent_watches(limit: i8)  {
    unimplemented!()
}

pub fn resume() {
    unimplemented!()
}

pub fn save_state() {
    unimplemented!()
}