use crate::db::Db;
use crate::indexer::{Episode, Tv};
use crate::library::{MediaLibrary, MediaType, PlaybackEvent, PlaybackState, SharedState};
use crate::mpv::Player;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::time::{sleep, timeout};

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

pub fn get_next_episode(current_episode_id: &str, episodes: &[Episode]) -> Option<Episode> {
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
        Player::start_monitoring(&player_clone, monitor_state, tx).await;
    });

    Ok((state, rx))
}

async fn play_next(
    current_media_id: &mut String,
    current_media_type: MediaType,
    index: &MediaLibrary,
    state: SharedState,
    player: Arc<Mutex<Player>>,
    has_reached_completion: &mut bool,
) -> Result<bool> {
    if current_media_type == MediaType::Show {
        if let Some((show, _, _)) = index.episode_map.get(current_media_id) {
            let episodes = flatten_show(show);
            if let Some(next_episode) = get_next_episode(&current_media_id, &episodes) {
                println!(
                    "{}",
                    format!("Playing next episode: {}", next_episode.name).blue()
                );
                *has_reached_completion = false;

                {
                    let mut state_guard = state.lock().await;
                    state_guard.init(
                        next_episode.id.clone(),
                        next_episode.path.clone(),
                        MediaType::Show,
                    );
                    state_guard.percent_pos = None;
                }

                {
                    let mut player_guard = player.lock().await;
                    if let Err(e) = player_guard.stop().await {
                        eprintln!("Failed to stop player: {}", e);
                    }
                }

                sleep(Duration::from_millis(1000)).await;

                {
                    let mut player_guard = player.lock().await;
                    if let Err(e) = player_guard.play_file(&next_episode.path).await {
                        eprintln!("Failed to load next file: {}", e);
                    }
                }

                *current_media_id = next_episode.id.clone();
                return Ok(true);
            } else {
                println!("{}", "No more episodes in this season".yellow());
                return Ok(false);
            }
        }
    }
    Ok(false)
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
                PlaybackEvent::Started => {
                    println!("{}", "▶️ Playback started".cyan());
                    has_reached_completion = false;
                }
                PlaybackEvent::Position(p, event_media_id) => {
                    if event_media_id != current_media_id {
                        println!(
                            "Skipping event for media_id: {} (current: {})",
                            event_media_id, current_media_id
                        );
                        continue;
                    }
                    {
                        let mut state_guard = state.lock().await;
                        state_guard.update_position(p);
                    }

                    if p > 98.0
                        && !has_reached_completion
                        && state.lock().await.media_id.as_ref() == Some(&current_media_id)
                    {
                        has_reached_completion = true;
                        println!("{}", "Reached 98% completion".cyan());
                        {
                            let mut state_guard = state.lock().await;
                            state_guard.mark_completed();
                        }

                        println!("{}", "Saving to db".cyan());
                        db.save_playback_progress(state.clone())
                            .await
                            .expect("Could not save playback progress");

                        // Drain queue to clear stale events
                        let drain_timeout = timeout(Duration::from_millis(500), async {
                            while rx.try_recv().is_ok() {}
                        });
                        if let Err(e) = drain_timeout.await {
                            println!("Queue drain timed out: {}", e);
                        }

                        // Play next episode
                        if play_next(
                            &mut current_media_id,
                            current_media_type.clone(),
                            index,
                            state.clone(),
                            player.clone(),
                            &mut has_reached_completion,
                        )
                        .await?
                        {
                            continue; // Continue to process events for the new episode
                        } else {
                            return Ok(());
                        }
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
