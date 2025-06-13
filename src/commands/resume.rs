use crate::commands::shared::{monitor_playback, start_playback};
use crate::db;
use crate::library::{MediaLibrary, MediaType};
use anyhow::Result;

pub async fn execute(index: &MediaLibrary) -> Result<()> {
    let db = db::Db::get();
    let incomplete: Vec<_> = db
        .get_recent_watches(10)
        .await?
        .into_iter()
        .filter(|entry| !entry.complete)
        .collect();

    if incomplete.is_empty() {
        println!("No incomplete watches to resume.");
        return Ok(());
    }

    let to_resume = &incomplete[0];
    match to_resume.media_type {
        MediaType::Movie => {
            if let Some(movie) = index.library.movies.iter().find(|m| m.id == to_resume.media_id) {
                println!("Resuming movie: {}", movie.name);
                let (_, rx) = start_playback(
                    movie.id.clone(),
                    movie.path.clone(),
                    MediaType::Movie,
                )
                    .await?;
                monitor_playback(rx, MediaType::Movie, movie.id.clone(), index).await?;
            }
        }
        MediaType::Show => {
            if let Some((_, _, episode)) = index.episode_map.get(&to_resume.media_id) {
                println!("Resuming episode: {}", episode.name);
                let (_, rx) = start_playback(
                    episode.id.clone(),
                    episode.path.clone(),
                    MediaType::Show,
                )
                    .await?;
                monitor_playback(rx, MediaType::Show, episode.id.clone(), index).await?;
            }
        }
    }

    Ok(())
}