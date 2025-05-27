#![allow(warnings)]
use crate::indexer::Library;
use crate::mpv::Player;
use clap::{Parser, Subcommand};
use colored::Colorize;
use crate::library::get_recent_watches;

#[derive(Parser)]
#[command(name = "pmc")]
#[command(about = "simple CLI tool to manage your media", long_about = None)]
pub struct App {
    #[arg(
        long,
        help = "Path to the movies directory",
        default_value = "/home/zeus/Downloads/media/movies"
    )]
    pub movie_dir: String,
    #[arg(
        long,
        help = "Path to the TV shows directory",
        default_value = "/home/zeus/Downloads/media/tv"
    )]
    pub tv_dir: String,
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
    mpv_socket: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut player = Player::init(mpv_socket).await?;
    Ok(())
}

pub async fn handle_play_command (index: &Library, mpv_socket: &str) -> Result<(), Box<dyn std::error::Error>> {
    unimplemented!()
}
