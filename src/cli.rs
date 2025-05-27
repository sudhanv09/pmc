#![allow(warnings)]
use crate::indexer::Library;
use crate::library::get_recent_watches;
use crate::mpv::Player;
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{MultiSelect, Select};
use crate::mpv;

#[derive(Parser)]
#[command(name = "pmc")]
#[command(about = "simple CLI tool to manage your media", long_about = None)]
pub struct App {
    #[arg(
        long,
        help = "Path to the MPV IPC socket",
        default_value = "/tmp/mpvipc"
    )]
    pub mpv_socket: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List,
    Recent,
    Play,
    Resume,
}

pub async fn handle_list_command(index: &Library) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Movies".green());
    println!("------");
    for item in &index.movies {
        println!("{}", &item.name);
    }
    println!("\n");

    println!("{}", "Shows".green());
    println!("------");
    for item in &index.shows {
        println!("{}", &item.name);
    }
    Ok(())
}

pub async fn handle_recent_command() -> Result<(), Box<dyn std::error::Error>> {
    let recent = get_recent_watches(5);

    Ok(())
}

pub async fn handle_resume_command(
    index: &Library,
    player: &Player
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub async fn handle_play_command(
    index: &Library,
    mut player: Player,
) -> Result<(), Box<dyn std::error::Error>> {
    let media_select = &["Movies", "Tv"];
    let selections = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick your food")
        .items(&media_select[..])
        .interact()
        .unwrap();

    println!("Select one of the {}", media_select[0]);

    match selections {
        0 => {
            if index.movies.is_empty() {
                println!("No movies found");
                return Ok(());
            }

            let movie_titles = &index
                .movies
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>();
            let choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Pick a movie to watch")
                .items(&movie_titles[..])
                .interact()
                .unwrap();
            
            let movie = &index.movies[choice];
            println!("Now playing movie: {}", movie.name);

            player.play_file(movie.path.clone()).await.expect("Unable to play file");
        }
        1 => {
            if index.shows.is_empty() {
                println!("No TV shows found.");
                return Ok(());
            }

            let show_titles: Vec<&str> = index.shows.iter().map(|s| s.name.as_str()).collect();

            let show_choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Pick a show")
                .items(&show_titles)
                .interact()
                .unwrap();

            let show = &index.shows[show_choice];
            println!("Now playing first episode of show: {}", show.name);
        }
        _ => {}
    };

    Ok(())
}
