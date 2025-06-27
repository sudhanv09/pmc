use crate::commands::shared::{monitor_playback, start_playback};
use crate::db;
use crate::library::MediaLibrary;
use crate::mpv::init_player;
use crate::state::MediaType;
use anyhow::Result;

pub async fn execute(index: &MediaLibrary) -> Result<()> {
    let db = db::Db::get();
    let recent: Vec<_> = db.get_recent_watches(10).await?;

    let player = init_player().await;

    // Resume or play next
    if let Some(to_resume) = recent.iter().find(|e| !e.complete) {
        match to_resume.media_type {
            MediaType::Movie => {
                if let Some(movie) = index
                    .library
                    .movies
                    .iter()
                    .find(|m| m.id == to_resume.media_id)
                {
                    println!("Resuming movie: {}", movie.name);
                    let (state, rx) = start_playback(
                        movie.id.clone(),
                        movie.path.clone(),
                        MediaType::Movie,
                        player.clone(),
                        Some(to_resume.progress as f64),
                    )
                    .await?;
                    monitor_playback(
                        rx,
                        MediaType::Movie,
                        movie.id.clone(),
                        index,
                        state,
                        db,
                        player.clone(),
                    )
                    .await?;
                }
            }
            MediaType::Show => {
                if let Some((_, _, episode)) = index.episode_map.get(&to_resume.media_id) {
                    println!("Resuming episode: {}", episode.name);
                    let (state, rx) = start_playback(
                        episode.id.clone(),
                        episode.path.clone(),
                        MediaType::Show,
                        player.clone(),
                        Some(to_resume.progress as f64),
                    )
                    .await?;
                    monitor_playback(
                        rx,
                        MediaType::Show,
                        episode.id.clone(),
                        index,
                        state,
                        db,
                        player.clone(),
                    )
                    .await?;
                }
            }
        }
    } else if let Some(latest) = recent.first() {
        if latest.media_type == MediaType::Show {
            if let Some(next_episode) = index.get_next_episode(&latest.media_id) {
                println!("Playing next episode: {}", next_episode.name);
                let (state, rx) = start_playback(
                    next_episode.id.clone(),
                    next_episode.path.clone(),
                    MediaType::Show,
                    player.clone(),
                    None,
                )
                .await?;
                monitor_playback(
                    rx,
                    MediaType::Show,
                    next_episode.id.clone(),
                    index,
                    state,
                    db,
                    player.clone(),
                )
                .await?;
            } else {
                println!("No more episodes available.");
            }
        } else {
            println!("No recent watches found.");
        }
    }

    Ok(())
}
