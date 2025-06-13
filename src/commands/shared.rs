use crate::indexer::{Episode, Tv};
use crate::library::{MediaLibrary, MediaType, PlaybackEvent, PlaybackState, SharedState};
use crate::mpv::{Player, spawn_mpv};
use colored::Colorize;
use std::path::Path;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[derive(Debug)]
pub enum PlaybackOutcome {
    Complete,
    Continue(String), // Next episode ID to play
}

type Result<T> = std::result::Result<T, anyhow::Error>;

pub fn flatten_show(show: &Tv) -> Vec<Episode> {
    let mut entries = Vec::new();

    for item in show.seasons.iter() {
        for episode in item.episodes.iter() {
            entries.push(episode.clone());
        }
    }

    entries
}

pub fn get_next_episode<'a>(
    current_episode_id: &str,
    episodes: &'a [Episode],
) -> Option<&'a Episode> {
    let current_pos = episodes.iter().position(|e| e.id == current_episode_id)?;
    episodes.get(current_pos + 1)
}

pub async fn start_playback(
    media_id: String,
    path: impl AsRef<Path>,
    media_type: MediaType,
) -> Result<(SharedState, UnboundedReceiver<PlaybackEvent>)> {
    let state: SharedState = std::sync::Arc::new(std::sync::Mutex::new(PlaybackState::new()));

    {
        let mut guard = state.lock().unwrap();
        guard.init(media_id, path.as_ref().to_path_buf(), media_type);
    }

    let socket_path = "/tmp/pmc-mpv.sock";
    spawn_mpv(socket_path)?;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("Started playback");
    
    let mut player = Player::init(socket_path).await?;
    player.play_file(path).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    
    let (tx, rx) = unbounded_channel::<PlaybackEvent>();
    let monitor_state = state.clone();

    tokio::spawn(async move {
        player.start_monitoring(monitor_state, tx).await;
    });
    
    Ok((state, rx))
}

pub async fn monitor_playback(
    mut rx: UnboundedReceiver<PlaybackEvent>,
    media_type: MediaType,
    media_id: String,
    index: &MediaLibrary,
    state: SharedState,
) -> Result<PlaybackOutcome> {
    let current_media_id = media_id;
    let current_media_type = media_type;
    let mut has_reached_completion = false;

    loop {
        while let Some(event) = rx.recv().await {
            match event {
                PlaybackEvent::Paused => println!("{}", "⏸ Paused".yellow()),
                PlaybackEvent::Resumed => println!("{}", "▶️ Resumed".green()),
                PlaybackEvent::Started => println!("{}", "▶️ Playback started".cyan()),
                PlaybackEvent::Position(p) => {
                    println!("⏱ {:.1}% played", p);

                    if p > 96.0 && !has_reached_completion {
                        has_reached_completion = true;
                        println!("{}", "Reached 95% completion".red());
                        {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.mark_completed(); // You'll need to add this method
                        }
                    }

                    if p > 96.0 && current_media_type == MediaType::Show {
                        if let Some((show, _, _)) = index.episode_map.get(&current_media_id) {
                            let episodes = flatten_show(show);
                            if let Some(next_episode) =
                                get_next_episode(&current_media_id, &episodes)
                            {
                                println!("{}", "Queueing next episode...".blue());
                                return Ok(PlaybackOutcome::Continue(next_episode.id.clone()));
                            }
                        }
                    }
                }
                PlaybackEvent::Completed => {
                    println!("{}", "✅ Playback complete".blue());
                    if !has_reached_completion {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.mark_completed();
                    }
                    break;
                }
                PlaybackEvent::Stopped => {
                    println!("{}", "🛑 Playback stopped".red());
                    break;
                }
                PlaybackEvent::Error(e) => {
                    eprintln!("❌ Playback error: {}", e);
                    break;
                }
            }
        }
    }
}
