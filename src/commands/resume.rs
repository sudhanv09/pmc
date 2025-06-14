use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use crate::commands::shared::{monitor_playback, start_playback};
use crate::db;
use crate::library::{MediaLibrary, MediaType};
use anyhow::Result;
use tokio::sync::Mutex;
use crate::mpv::{spawn_mpv, Player};

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
    let socket_name = "/tmp/pmc-mpv.sock";
    spawn_mpv(socket_name).expect("Failed to spawn MPV");
    sleep(Duration::from_secs(1));

    let player = Arc::new(Mutex::new(Player::init(socket_name).await?));
    
    match to_resume.media_type {
        MediaType::Movie => {
            if let Some(movie) = index.library.movies.iter().find(|m| m.id == to_resume.media_id) {
                println!("Resuming movie: {}", movie.name);
                
                let (state, rx) = start_playback(
                    movie.id.clone(),
                    movie.path.clone(),
                    MediaType::Movie,
                    player.clone(),
                )
                    .await?;
                monitor_playback(rx, MediaType::Movie, movie.id.clone(), index, state, db, player.clone()).await?;
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
                )
                    .await?;
                monitor_playback(rx, MediaType::Show, episode.id.clone(), index, state, db, player.clone()).await?;
            }
        }
    }

    Ok(())
}