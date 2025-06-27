use crate::indexer;
use crate::indexer::{Episode, Library, Season, Tv};
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

pub async fn load_or_configure_library() -> Result<MediaLibrary> {
    let index_path = "./index.json";

    if Path::new(index_path).exists() {
        let data = fs::read_to_string(index_path)?;
        let mut library: MediaLibrary = serde_json::from_str(&data)?;
        library.rebuild_maps();
        Ok(library)
    } else {
        println!("{}", "Welcome to pmc!".blue());

        let movie_dir: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter movie directory")
            .interact_text()
            .context("Failed to read movie directory")?;

        let tv_dir: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter tv directory")
            .interact_text()
            .context("Failed to read shows directory")?;

        let library = indexer::index(movie_dir.clone(), tv_dir.clone());
        let media_library = MediaLibrary::new(movie_dir, tv_dir, library);

        let serialized = serde_json::to_string_pretty(&media_library)?;
        fs::write(index_path, serialized)?;

        Ok(media_library)
    }
}
