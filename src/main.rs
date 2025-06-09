use std::fs;
use std::path::Path;
use chrono::{DateTime, Local};
use crate::cli::{App, Commands, handle_list_command, handle_play_command, handle_resume_command, handle_recent_command};
use clap::Parser;
use colored::Colorize;
use dialoguer::Input;
use dialoguer::theme::ColorfulTheme;
use serde::{Deserialize, Serialize};
use crate::indexer::Library;

mod cli;
mod db;
mod indexer;
mod library;
mod mpv;
mod utils;


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LibraryIndex {
    pub movie_dir: String,
    pub tv_dir: String,
    pub indexed_at: DateTime<Local>,
    pub library: Library,
}

async fn configure_user() -> tokio::io::Result<LibraryIndex> {
    let index_path = "./index.json";

    if Path::new(index_path).exists() {
        let data = fs::read_to_string(index_path)?;
        let library: LibraryIndex = serde_json::from_str(&data)?;
        Ok(library)
    } else {
        println!("{}", "Welcome to pmc!".blue());
        let movie_dir: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter movie directory")
            .interact_text()
            .unwrap();

        let tv_dir: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter tv directory")
            .interact_text()
            .unwrap();

        let library = indexer::index(movie_dir.clone(), tv_dir.clone());
        let index = LibraryIndex {
            movie_dir,
            tv_dir,
            indexed_at: Local::now(),
            library,
        };
        let serialized = serde_json::to_string_pretty(&index)?;
        fs::write(index_path, serialized)?;

        Ok(index)
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let index = configure_user().await?;

    db::Db::init("./pmc.db").await.map_err(|e| {
        eprintln!("Database initialization failed: {}", e);
        std::process::exit(1);
    }).unwrap();


    let cli = App::parse();
    let mpv = mpv::Player::init(&cli.mpv_socket).await.expect("Is mpv running? Run it with mpv --force-window --idle --input-ipc-server=<socketname>");

    let result = match &cli.command {
        Commands::List => handle_list_command(&index.library).await,
        Commands::Recent => handle_recent_command(&index.library).await,

        Commands::Play => handle_play_command(&index.library, mpv).await,
        Commands::Resume => handle_resume_command(&index.library, &mpv).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        // exit with a non-zero code here
        std::process::exit(1);
    }

    Ok(())
}
