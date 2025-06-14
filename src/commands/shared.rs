use crate::db::Db;
use crate::indexer::{Episode, Tv};
use crate::library::{MediaLibrary, MediaType, PlaybackEvent, PlaybackState, SharedState};
use crate::mpv::{Player};
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

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

pub fn get_next_episode(
    current_episode_id: &str,
    episodes: &[Episode],
) -> Option<Episode> {
    let current_pos = episodes.iter().position(|e| e.id == current_episode_id)?;
    episodes.get(current_pos + 1).cloned()
}

pub async fn start_playback(
    media_id: String,
    path: impl AsRef<Path>,
    media_type: MediaType,
    player: Arc<Mutex<Player>>,
) -> Result<(SharedState, UnboundedReceiver<PlaybackEvent>)> {
    let state: SharedState = Arc::new(Mutex::new(PlaybackState::new()));
    {
        let mut guard = state.lock().await;
        guard.init(media_id, path.as_ref().to_path_buf(), media_type);
    }

    {
        let mut player_guard = player.lock().await;
        player_guard.play_file(path).await?;
    }

    let (tx, rx) = unbounded_channel::<PlaybackEvent>();
    let monitor_state = state.clone();
    let player_clone = player.clone();

    tokio::spawn(async move {
        let mut player_guard = player_clone.lock().await;
        player_guard.start_monitoring(monitor_state, tx).await;
    });

    Ok((state, rx))
}

pub async fn monitor_playback(
    mut rx: UnboundedReceiver<PlaybackEvent>,
    media_type: MediaType,
    media_id: String,
    index: &MediaLibrary,
    state: SharedState,
    db: &Db,
    player: Arc<Mutex<Player>>,
) -> Result<()> {
    let mut current_media_id = media_id.clone();
    let current_media_type = media_type;
    let mut has_reached_completion = false;

    loop {
        while let Some(event) = rx.recv().await {
            match event {
                PlaybackEvent::Paused => println!("{}", "⏸ Paused".yellow()),
                PlaybackEvent::Resumed => println!("{}", "▶️ Resumed".green()),
                PlaybackEvent::Started => println!("{}", "▶️ Playback started".cyan()),
                PlaybackEvent::Position(p) => {
                    if p > 96.0 && !has_reached_completion {
                        has_reached_completion = true;
                        println!("{}", "Reached 95% completion".red());
                        let mut state_guard = state.lock().await;
                        state_guard.mark_completed();
                        db.save_playback_progress(state.clone())
                            .await
                            .expect("Could not save playback progress");
                    }
                }
                PlaybackEvent::Completed => {
                    println!("{}", "✅ Playback complete".blue());
                    if !has_reached_completion {
                        let mut state_guard = state.lock().await;
                        state_guard.mark_completed();
                        db.save_playback_progress(state.clone())
                            .await
                            .expect("Could not save playback progress");
                    }
                    if current_media_type == MediaType::Show {
                        if let Some((show, _, _)) = index.episode_map.get(&media_id) {
                            let episodes = flatten_show(show);
                            if let Some(next_episode) = get_next_episode(&current_media_id, &episodes) {
                                println!(
                                    "{}",
                                    format!("Playing next episode: {}", next_episode.name).blue()
                                );
                                let (new_state, new_rx) = start_playback(
                                    next_episode.id.clone(),
                                    next_episode.path.clone(),
                                    MediaType::Show,
                                    player.clone(),
                                )
                                .await?;
                                current_media_id = next_episode.id;
                                *state.lock().await = new_state.lock().await.clone();
                                rx = new_rx;
                                has_reached_completion = false;
                                continue;
                            } else {
                                println!("{}", "No more episodes in this season".yellow());
                                return Ok(());
                            }
                        }
                    }
                    return Ok(());
                }
                PlaybackEvent::Stopped => {
                    println!("{}", "🛑 Playback stopped".red());
                    break;
                }
                PlaybackEvent::Exited => {
                    println!("{}", "🛑 MPV has exited, terminating application".red());
                    std::process::exit(0);
                }
                PlaybackEvent::Error(e) => {
                    eprintln!("❌ Playback error: {}", e);
                    break;
                }
            }
        }
    }
}
