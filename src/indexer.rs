use crate::utils::{guess_name, guess_season};
use chrono::{DateTime, Local};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Library {
    pub movies: Vec<Movie>,
    pub shows: Vec<Tv>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Movie {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Local>,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Tv {
    pub id: String,
    pub name: String,
    pub seasons: Vec<Season>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Season {
    pub id: String,
    pub name: String,
    pub number: i32,
    pub episodes: Vec<Episode>,
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Episode {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub created_at: DateTime<Local>,
    pub size: u64,
}

impl Tv {
    pub fn flatten_show(&self) -> Vec<Episode> {
        let mut entries = Vec::new();

        for item in self.seasons.iter() {
            for episode in item.episodes.iter() {
                entries.push(episode.clone());
            }
        }

        entries
    }
}

fn valid_ext(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("mkv") | Some("mp4") | Some("avi")
    )
}
fn index_movies(dir: String) -> Vec<Movie> {
    let mut movies: Vec<Movie> = WalkDir::new(dir)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            if !valid_ext(path) {
                return None;
            }

            let name = guess_name(path.file_stem().and_then(|s| s.to_str())?);
            let metadata = entry.metadata().ok();
            let created_at: DateTime<Local> = metadata
                .as_ref()
                .and_then(|m| m.created().or_else(|_| m.modified()).ok())
                .map(|t| t.into())
                .unwrap_or_else(Local::now);
            let size = metadata.map(|m| m.len()).unwrap_or(0);

            Some(Movie {
                id: nanoid!(10),
                name: name.to_string(),
                path: path.to_path_buf(),
                created_at,
                size,
            })
        })
        .collect();

    movies.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    movies
}

fn index_shows(dir: String) -> Vec<Tv> {
    let mut shows = vec![];

    for entry in WalkDir::new(&dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
    {
        let show_path = entry.path();
        let show_name = guess_name(
            show_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Show"),
        );

        let mut seasons_map: HashMap<i32, Season> = HashMap::new();

        let subdirs: Vec<_> = WalkDir::new(show_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .collect();

        if !subdirs.is_empty() {
            // Case: Show -> Season Dir -> Episodes
            for season_entry in subdirs {
                let season_path = season_entry.path();
                let season_name = season_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Season");
                let season_number = guess_season(season_name);

                let mut episodes = vec![];

                for file_entry in WalkDir::new(season_path)
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_file())
                {
                    let path = file_entry.path();
                    if !valid_ext(path) {
                        continue;
                    }

                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unnamed Episode")
                        .to_string();
                    let metadata = file_entry.metadata().ok();
                    let created_at: DateTime<Local> = metadata
                        .as_ref()
                        .and_then(|m| m.created().or_else(|_| m.modified()).ok())
                        .map(|t| t.into())
                        .unwrap_or_else(Local::now);
                    let size = metadata.map(|m| m.len()).unwrap_or(0);

                    episodes.push(Episode {
                        id: nanoid!(10),
                        name,
                        path: path.to_path_buf(),
                        created_at,
                        size,
                    });
                }

                if !episodes.is_empty() {
                    episodes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    seasons_map.insert(
                        season_number,
                        Season {
                            id: nanoid!(10),
                            name: season_name.to_string(),
                            number: season_number,
                            episodes,
                        },
                    );
                }
            }
        } else {
            // Case: Show -> Episodes directly
            let mut episode_map: HashMap<i32, Vec<Episode>> = HashMap::new();

            for file_entry in WalkDir::new(show_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
            {
                let path = file_entry.path();
                if !valid_ext(path) {
                    continue;
                }

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unnamed Episode")
                    .to_string();
                let metadata = file_entry.metadata().ok();
                let created_at = metadata
                    .as_ref()
                    .and_then(|m| m.created().or_else(|_| m.modified()).ok())
                    .map(|t| t.into())
                    .unwrap_or_else(Local::now);
                let size = metadata.map(|m| m.len()).unwrap_or(0);

                let season_number = guess_season(&name);
                episode_map.entry(season_number).or_default().push(Episode {
                    id: nanoid!(10),
                    name,
                    path: path.to_path_buf(),
                    created_at,
                    size,
                });
            }

            for (season_number, mut episodes) in episode_map {
                episodes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                seasons_map.insert(
                    season_number,
                    Season {
                        id: nanoid!(10),
                        name: format!("Season {}", season_number),
                        number: season_number,
                        episodes,
                    },
                );
            }
        }

        let mut seasons: Vec<_> = seasons_map.into_values().collect();
        seasons.sort_by_key(|s| s.number);

        if !seasons.is_empty() {
            shows.push(Tv {
                id: nanoid!(10),
                name: show_name,
                seasons,
            });
        }
    }

    shows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    shows
}

pub fn index(movie_dir: String, show_dir: String) -> Library {
    Library {
        movies: index_movies(movie_dir),
        shows: index_shows(show_dir),
    }
}
