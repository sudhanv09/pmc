use crate::db::Db;
use crate::library::MediaLibrary;
use crate::mpv::Player;
use crate::state::{MediaType, PlaybackEvent, PlaybackState, SharedState};
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

pub async fn start_playback(
    media_id: String,
    path: impl AsRef<Path>,
    media_type: MediaType,
    player: Arc<Mutex<Player>>,
    percent_pos: Option<f64>,
) -> Result<(SharedState, UnboundedReceiver<PlaybackEvent>)> {
    let state: SharedState = Arc::new(Mutex::new(PlaybackState::new()));
    {
        let mut guard = state.lock().await;
        guard.init(media_id, path.as_ref().to_path_buf(), media_type);
    }

    {
        let mut player_guard = player.lock().await;
        player_guard.play_file(path).await?;
        if let Some(pos) = percent_pos {
            if !player_guard.wait_mpv().await {
                player_guard.seek(pos).await?;
            } else {
                println!("Failed to initialize playback for seek");
            }
        }
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
        if let Some(next_episode) = index.get_next_episode(current_media_id) {
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
    Ok(false)
}

pub async fn acquire_get_state(state: &SharedState) -> (String, MediaType, f64, bool) {
    let state_guard = state.lock().await;
    let media_id = state_guard.media_id.clone();
    let media_type = state_guard.media_type.clone();
    let progress = state_guard.completion_percentage();
    let is_completed = state_guard.is_completed();

    if progress > 98.0 && !is_completed {
        drop(state_guard); // Release lock before re-acquiring
        let mut state_guard = state.lock().await;
        state_guard.mark_completed();
    }

    (
        media_id.unwrap(),
        media_type.unwrap(),
        progress,
        is_completed,
    )
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

                        let (media_id, media_type, progress, is_completed) =
                            acquire_get_state(&state).await;

                        println!("{}", "Saving to db".cyan());
                        db.save_playback_progress(media_id, media_type, progress, is_completed)
                            .await
                            .expect("Could not save playback progress");

                        // Drain queue to clear stale events
                        // let drain_timeout = timeout(Duration::from_millis(500), async {
                        //     while rx.try_recv().is_ok() {}
                        // });
                        // if let Err(e) = drain_timeout.await {
                        //     println!("Queue drain timed out: {}", e);
                        // }

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
                        let (media_id, media_type, progress, is_completed) =
                            acquire_get_state(&state).await;
                        db.save_playback_progress(media_id, media_type, progress, is_completed)
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
                PlaybackEvent::RequestNext => {
                    if current_media_type == MediaType::Show {
                        if let Some(next_episode) = index.get_next_episode(&current_media_id) {
                            println!(
                                "{}",
                                format!("Playing next episode: {}", next_episode.name).blue()
                            );
                            has_reached_completion = false;
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
                                if let Err(e) = player_guard.playback_next(&next_episode.path).await
                                {
                                    eprintln!("Failed to play next episode: {}", e);
                                }
                            }
                            current_media_id = next_episode.id.clone();
                            continue;
                        } else {
                            println!("{}", "No more episodes in this season".yellow());
                        }
                    }
                }
                PlaybackEvent::RequestPrev => {
                    if current_media_type == MediaType::Show {
                        if let Some(prev_episode) = index.get_prev_episode(&current_media_id) {
                            println!(
                                "{}",
                                format!("Playing previous episode: {}", prev_episode.name).blue()
                            );
                            has_reached_completion = false;
                            {
                                let mut state_guard = state.lock().await;
                                state_guard.init(
                                    prev_episode.id.clone(),
                                    prev_episode.path.clone(),
                                    MediaType::Show,
                                );
                                state_guard.percent_pos = None;
                            }
                            {
                                let mut player_guard = player.lock().await;
                                if let Err(e) = player_guard.stop().await {
                                    eprintln!("Failed to stop player: {}", e);
                                }
                                if let Err(e) = player_guard.playback_prev(&prev_episode.path).await
                                {
                                    eprintln!("Failed to play previous episode: {}", e);
                                }
                            }
                            current_media_id = prev_episode.id.clone();
                            continue;
                        } else {
                            println!("{}", "No previous episodes available".yellow());
                        }
                    }
                }
                PlaybackEvent::RequestQuit => {
                    println!("{}", "User requested quit".cyan());
                    let (media_id, media_type, progress, is_completed) =
                        acquire_get_state(&state).await;
                    if !media_id.is_empty() {
                        db.save_playback_progress(media_id, media_type, progress, is_completed)
                            .await
                            .expect("Could not save playback progress");
                    }
                    {
                        let mut player_guard = player.lock().await;
                        if let Err(e) = player_guard.user_quit().await {
                            eprintln!("Failed to quit MPV: {}", e);
                        }
                    }
                    let _ = rx.try_recv(); // Clear any pending events
                    break;
                }
            }
        }
    }
}
