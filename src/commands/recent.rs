use crate::db;
use crate::library::{MediaLibrary, MediaType};
use anyhow::Result;
use colored::Colorize;

pub async fn execute(index: &MediaLibrary) -> Result<()> {
    let db = db::Db::get();
    let recent = db.get_recent_watches(5).await?;

    if recent.is_empty() {
        println!("{}", "No recent watches found.".yellow());
        return Ok(());
    }

    println!("{}", "Recent watches found.".green());
    for item in &recent {
        match item.media_type {
            MediaType::Movie => {
                if let Some(movie) = index.library.movies.iter().find(|m| m.id == item.media_id) {
                    println!(
                        "{} | {} | Progress: {}% | Completed: {}",
                        movie.name.cyan(),
                        item.watched_at.format("%Y-%m-%d %H:%M"),
                        item.progress,
                        if item.complete { "Yes" } else { "No" }
                    );
                }
            }
            MediaType::Show => {
                if let Some((tv, season, episode)) = index.episode_map.get(&item.media_id) {
                    println!(
                        "{} (S{} E{}) \n\t {} \t Progress: {}% \t Completed: {}",
                        tv.name.cyan(),
                        season.number,
                        episode.name,
                        item.watched_at.format("%Y-%m-%d %H:%M"),
                        item.progress,
                        if item.complete { "Yes" } else { "No" }
                    );
                }
            }
        }
    }
    Ok(())
}