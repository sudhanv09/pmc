use crate::db::Db;
use crate::indexer::Episode;
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

    let (tx, rx) = unbounded_channel();
    let monitor_state = state.clone();
    let player_clone = player.clone();

    tokio::spawn(async move {
        Player::start_monitoring(&player_clone, monitor_state, tx).await;
    });

    Ok((state, rx))
}

async fn transition_to_episode(
    episode_id: String,
    episode_path: std::path::PathBuf,
    current_media_id: &mut String,
    state: &SharedState,
    player: &Arc<Mutex<Player>>,
) -> Result<()> {
    {
        let mut state_guard = state.lock().await;
        state_guard.init(episode_id.clone(), episode_path.clone(), MediaType::Show);
        state_guard.percent_pos = None;
    }

    {
        let mut player_guard = player.lock().await;
        player_guard.stop().await.ok();
        player_guard.play_file(&episode_path).await?;
    }

    *current_media_id = episode_id;
    Ok(())
}

async fn play_next(
    current_media_id: &mut String,
    current_media_type: MediaType,
    index: &MediaLibrary,
    state: SharedState,
    player: Arc<Mutex<Player>>,
    has_reached_completion: &mut bool,
) -> Result<bool> {
    if current_media_type != MediaType::Show {
        return Ok(false);
    }

    if let Some(next_episode) = index.get_next_episode(current_media_id) {
        println!(
            "{}",
            format!("Playing next episode: {}", next_episode.name).blue()
        );
        *has_reached_completion = false;

        transition_to_episode(
            next_episode.id.clone(),
            next_episode.path.clone(),
            current_media_id,
            &state,
            &player,
        )
        .await?;

        Ok(true)
    } else {
        println!("{}", "No more episodes in this season".yellow());
        Ok(false)
    }
}

pub async fn acquire_get_state(state: &SharedState) -> (String, MediaType, f64, bool) {
    let mut state_guard = state.lock().await;
    let media_id = state_guard.media_id.clone();
    let media_type = state_guard.media_type.clone();
    let progress = state_guard.completion_percentage();
    let is_completed = state_guard.is_completed();

    if progress > 98.0 && !is_completed {
        state_guard.mark_completed();
    }

    (
        media_id.unwrap(),
        media_type.unwrap(),
        progress,
        is_completed,
    )
}

async fn handle_episode_request(
    episode: Option<Episode>,
    direction: &str,
    current_media_id: &mut String,
    state: &SharedState,
    player: &Arc<Mutex<Player>>,
    has_reached_completion: &mut bool,
) {
    if let Some(episode) = episode {
        println!(
            "{}",
            format!("Playing {} episode: {}", direction, episode.name).blue()
        );
        *has_reached_completion = false;

        transition_to_episode(
            episode.id.clone(),
            episode.path.clone(),
            current_media_id,
            state,
            player,
        )
        .await
        .unwrap_or_else(|e| eprintln!("Failed to play {} episode: {}", direction, e));
    } else {
        println!(
            "{}",
            format!("No {} episodes available", direction).yellow()
        );
    }
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
                PlaybackEvent::Position(p, ref id) if id == &current_media_id => {
                    state.lock().await.update_position(p);

                    if p > 98.0 && !has_reached_completion {
                        has_reached_completion = true;
                        println!("{}", "Reached 98% completion".cyan());

                        let (media_id, media_type, progress, is_completed) =
                            acquire_get_state(&state).await;

                        println!("{}", "Saving to db".cyan());
                        db.save_playback_progress(media_id, media_type, progress, is_completed)
                            .await?;

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
                            continue;
                        } else {
                            return Ok(());
                        }
                    }
                }
                PlaybackEvent::Position(_, _) => {
                    println!(
                        "Skipping event for media_id: {} (current: {})",
                        &media_id, current_media_id
                    );
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
                        let next_episode = index.get_next_episode(&current_media_id);
                        handle_episode_request(
                            next_episode,
                            "next",
                            &mut current_media_id,
                            &state,
                            &player,
                            &mut has_reached_completion,
                        )
                        .await;
                    }
                }
                PlaybackEvent::RequestPrev => {
                    if current_media_type == MediaType::Show {
                        let prev_episode = index.get_prev_episode(&current_media_id);
                        handle_episode_request(
                            prev_episode,
                            "previous",
                            &mut current_media_id,
                            &state,
                            &player,
                            &mut has_reached_completion,
                        )
                            .await;
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
