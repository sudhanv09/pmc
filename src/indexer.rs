use chrono::{DateTime, Local};
use nanoid::nanoid;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
}

fn valid_ext(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("mkv") | Some("mp4") | Some("avi")
    )
}

fn index_movie(dir: String) -> Vec<Movie> {
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

fn index_show(dir: String) -> Vec<Tv> {
    let mut shows = vec![];
    shows
}

pub fn index(movie_dir: String, show_dir: String) -> Library {
    let library = Library {
        movies: index_movie(movie_dir),
        shows: index_show(show_dir),
    };

    library
}

pub fn update_index() -> Library {
    let library = Library {
        movies: vec![],
        shows: vec![],
    };

    library
}
