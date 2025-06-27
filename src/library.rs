use crate::indexer;
use crate::indexer::{Episode, Library, Season, Tv};
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use colored::Colorize;
use dialoguer::Input;
use dialoguer::theme::ColorfulTheme;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path};

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
        let indexed_at = Local::now();
        let (show_map, episode_map) = Self::build_maps(&library);

        Self {
            movie_dir,
            tv_dir,
            indexed_at,
            library,
            show_map,
            episode_map,
        }
    }

    fn build_maps(
        library: &Library,
    ) -> (HashMap<String, Tv>, HashMap<String, (Tv, Season, Episode)>) {
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

        (show_map, episode_map)
    }

    pub fn rebuild_maps(&mut self) {
        let (show_map, episode_map) = Self::build_maps(&self.library);
        self.show_map = show_map;
        self.episode_map = episode_map;
    }

    pub fn get_episode(&self, episode_id: &str) -> Option<&(Tv, Season, Episode)> {
        self.episode_map.get(episode_id)
    }

    pub fn get_show(&self, show_id: &str) -> Option<&Tv> {
        self.show_map.get(show_id)
    }

    pub fn get_next_episode(&self, current_episode_id: &str) -> Option<Episode> {
        let (_, season, episode) = self.episode_map.get(current_episode_id)?;
        let episodes = &season.episodes;

        let current_pos = episodes.iter().position(|e| e.id == episode.id)?;
        episodes.get(current_pos + 1).cloned()
    }

    pub fn get_prev_episode(&self, current_episode_id: &str) -> Option<Episode> {
        let (_, season, episode) = self.episode_map.get(current_episode_id)?;
        let episodes = &season.episodes;

        let current_pos = episodes.iter().position(|e| e.id == episode.id)?;
        if current_pos > 0 {
            episodes.get(current_pos - 1).cloned()
        } else {
            None
        }
    }
}

pub async fn load_or_configure_library() -> Result<MediaLibrary> {
    let index_path = "./index.json";

    if let Ok(data) = fs::read_to_string(index_path) {
        let mut library: MediaLibrary = serde_json::from_str(&data)?;
        library.rebuild_maps();
        return Ok(library);
    }

    println!("{}", "Welcome to pmc!".blue());

    let (movie_dir, tv_dir) = prompt_for_dirs()?;
    let library = indexer::index(movie_dir.clone(), tv_dir.clone());
    let media_library = MediaLibrary::new(movie_dir, tv_dir, library);

    let serialized = serde_json::to_string_pretty(&media_library)?;
    fs::write(index_path, serialized)?;

    Ok(media_library)
}

fn validate_dir(path: &str) -> Result<String> {
    let normed = shellexpand::tilde(path).to_string();
    let path = Path::new(&normed);

    if !path.exists() {
        anyhow::bail!("path does not exist");
    }

    if !path.is_dir() {
        anyhow::bail!("path is not a directory");
    }

    let is_empty = fs::read_dir(path)?.next().is_none();
    if is_empty {
        println!(
            "{}",
            format!("Warning: {} is empty.", path.display()).yellow()
        );
    }

    let absolute = fs::canonicalize(path)?;
    Ok(absolute.to_string_lossy().into_owned())
}

fn prompt_for_dirs() -> Result<(String, String)> {
    let movie_dir = loop {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter movie directory")
            .interact_text()
            .context("Failed to read movie directory")?;

        if let Err(err) = validate_dir(&input) {
            println!("{}", format!("Invalid movie directory: {}", err).red());
        } else {
            break input;
        }
    };

    let tv_dir = loop {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter TV directory")
            .interact_text()
            .context("Failed to read TV directory")?;

        if let Err(err) = validate_dir(&input) {
            println!("{}", format!("Invalid TV directory: {}", err).red());
        } else {
            break input;
        }
    };

    Ok((movie_dir, tv_dir))
}
