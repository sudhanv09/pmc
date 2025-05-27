use chrono::{DateTime, Local};
use nanoid::nanoid;
use regex::Regex;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Library {
    pub movies: Vec<Movie>,
    pub shows: Vec<Tv>,
}

#[derive(Debug)]
pub struct Movie {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub created_at: String,
    pub size: u64,
}

#[derive(Debug)]
pub struct Tv {
    pub id: String,
    pub name: String,
    pub seasons: Vec<Season>,
}

#[derive(Debug)]
pub struct Season {
    pub id: String,
    pub name: String,
    pub number: i32,
    pub episodes: Vec<Episode>,
}
#[derive(Debug)]
pub struct Episode {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub created_at: String,
    pub size: u64,
}

fn valid_ext(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("mkv") | Some("mp4") | Some("avi")
    )
}

/// Extracts the season number from a file or directory name.
/// Returns 0 if not found.
fn guess_season(item: &str) -> i32 {
    let patterns = [
        Regex::new(r"(?i)S(\d{1,2})E\d{1,2}").unwrap(), // S01E02
        Regex::new(r"(?i)Season[ _]?(\d{1,2})").unwrap(), // Season 2
        Regex::new(r"(?i)S(\d{1,2})").unwrap(),         // S1
    ];

    for re in &patterns {
        if let Some(caps) = re.captures(item) {
            if let Some(season) = caps.get(1) {
                return season.as_str().parse().unwrap_or(0);
            }
        }
    }

    0
}

/// Extracts the episode number from a file or directory name.
/// Returns 0 if not found.
fn guess_episode(item: &str) -> i32 {
    let patterns = [
        Regex::new(r"(?i)S\d{1,2}E(\d{1,2})").unwrap(), // S01E02
        Regex::new(r"(?i)Episode[ _]?(\d{1,2})").unwrap(), // Episode 3
        Regex::new(r"(?i)E(\d{1,2})").unwrap(),         // E3
    ];

    for re in &patterns {
        if let Some(caps) = re.captures(item) {
            if let Some(ep) = caps.get(1) {
                return ep.as_str().parse().unwrap_or(0);
            }
        }
    }

    0
}

fn index_movies(dir: String) -> Vec<Movie> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            if !valid_ext(path) {
                return None;
            }

            let name = path.file_stem().and_then(|s| s.to_str())?;
            let metadata = entry.metadata().ok()?;

            let created_time = metadata.created().or_else(|_| metadata.modified()).ok()?;
            let datetime: DateTime<Local> = DateTime::from(created_time);
            let created_at_str = datetime.format("%d/%m/%Y %T").to_string();

            Some(Movie {
                id: nanoid!(10),
                name: name.to_string(),
                path: path.to_path_buf(),
                created_at: created_at_str,
                size: metadata.len(),
            })
        })
        .collect()
}

fn index_shows(dir: String) -> Vec<Tv> {
    let mut shows = vec![];

    for entry in WalkDir::new(&dir)
        .min_depth(1)
        .max_depth(2) // Show -> Season
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
    {
        let show_path = entry.path();
        let show_name = show_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Show")
            .to_string();
        let mut seasons = vec![];

        for season_entry in WalkDir::new(show_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
        {
            let season_path = season_entry.path();
            let season_name = season_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Season");

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
                let created_at = metadata
                    .as_ref()
                    .and_then(|m| m.created().or_else(|_| m.modified()).ok())
                    .map(|t| DateTime::<Local>::from(t).format("%d/%m/%Y %T").to_string())
                    .unwrap_or_default();
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
                seasons.push(Season {
                    id: nanoid!(10),
                    name: season_name.to_string(),
                    number: guess_season(season_name),
                    episodes,
                });
            }
        }

        if !seasons.is_empty() {
            shows.push(Tv {
                id: nanoid!(10),
                name: show_name,
                seasons,
            });
        }
    }

    shows
}

pub fn index(movie_dir: String, show_dir: String) -> Library {
    Library {
        movies: index_movies(movie_dir),
        shows: index_shows(show_dir),
    }
}

// TODO: how to handle updates?
pub fn update_index() -> Library {
    let library = Library {
        movies: vec![],
        shows: vec![],
    };

    library
}
