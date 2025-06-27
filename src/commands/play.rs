use crate::commands::shared::{acquire_get_state, flatten_show, monitor_playback, start_playback};
use crate::db;
use crate::library::{MediaLibrary, MediaType};
use crate::mpv::{Player, spawn_mpv};
use anyhow::Result;
use dialoguer::{Select, theme::ColorfulTheme};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn execute(index: &MediaLibrary) -> Result<()> {
    let db = db::Db::get();

    let media_select = &["Movies", "Tv"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What do you want to watch?")
        .items(&media_select[..])
        .interact()?;

    match selection {
        0 => play_movie(index, db).await,
        1 => play_show(index, db).await,
        _ => Ok(()),
    }
}

async fn play_movie(index: &MediaLibrary, db: &db::Db) -> Result<()> {
    if index.library.movies.is_empty() {
        println!("No movies found");
        return Ok(());
    }

    let movie_titles: Vec<&str> = index
        .library
        .movies
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a movie to watch")
        .items(&movie_titles)
        .interact()?;

    let movie = &index.library.movies[choice];
    println!("Now playing movie: {}", movie.name);

    let socket_name = "/tmp/pmc-mpv.sock";
    spawn_mpv(socket_name).expect("Failed to spawn MPV");

    let player = Arc::new(Mutex::new(Player::init(socket_name).await?));

    let (state, rx) = start_playback(
        movie.id.clone(),
        movie.path.clone(),
        MediaType::Movie,
        player.clone(),
        None,
    )
    .await?;

    monitor_playback(
        rx,
        MediaType::Movie,
        movie.id.clone(),
        index,
        state.clone(),
        db,
        player.clone(),
    )
    .await?;
    let (media_id, media_type, progress, is_completed) = acquire_get_state(&state).await;
    db.save_playback_progress(media_id, media_type, progress, is_completed)
        .await
        .unwrap();

    Ok(())
}

async fn play_show(index: &MediaLibrary, db: &db::Db) -> Result<()> {
    if index.library.shows.is_empty() {
        println!("No TV shows found.");
        return Ok(());
    }

    let show_titles: Vec<&str> = index
        .library
        .shows
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a show")
        .items(&show_titles)
        .interact()?;

    let show = &index.library.shows[choice];
    let episodes = flatten_show(show);
    let history = db.get_media_history(show.id.clone()).await?;

    if let Some(episode) = select_episode_to_play(&episodes, &history) {
        println!("Now playing: {}", episode.name);

        let socket_name = "/tmp/pmc-mpv.sock";
        spawn_mpv(socket_name).expect("Failed to spawn MPV");

        let player = Arc::new(Mutex::new(Player::init(socket_name).await?));

        let (state, rx) = start_playback(
            episode.id.clone(),
            episode.path.clone(),
            MediaType::Show,
            player.clone(),
            None,
        )
        .await?;

        monitor_playback(
            rx,
            MediaType::Show,
            episode.id.clone(),
            index,
            state.clone(),
            db,
            player.clone(),
        )
        .await?;
        let (media_id, media_type, progress, is_completed) = acquire_get_state(&state).await;
        db.save_playback_progress(media_id, media_type, progress, is_completed)
            .await
            .unwrap();
    } else {
        println!("Nothing to play.");
    }

    Ok(())
}

fn select_episode_to_play<'a>(
    episodes: &'a [crate::indexer::Episode],
    history: &[crate::library::WatchEntry],
) -> Option<&'a crate::indexer::Episode> {
    let last = history
        .iter()
        .filter(|entry| entry.media_type == MediaType::Show)
        .max_by_key(|entry| entry.watched_at);

    match last {
        None => episodes.first(),
        Some(entry) => {
            let idx = episodes.iter().position(|ep| ep.id == entry.media_id);
            match (idx, entry.complete) {
                (Some(i), false) => Some(&episodes[i]),
                (Some(i), true) => episodes.get(i + 1),
                _ => episodes.first(),
            }
        }
    }
}
