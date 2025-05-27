#![allow(warnings)]
use clap::{Parser, Subcommand};
use crate::indexer::Library;
use crate::mpv::Player;
use colored::Colorize;

#[derive(Parser)]
#[command(name = "pmc")]
#[command(about = "simple CLI tool to manage your media", long_about = None)]
pub struct App {
    #[arg(long, help = "Path to the movies directory", default_value = "/home/zeus/Downloads/media/movies")]
    pub movie_dir: String,
    #[arg(long, help = "Path to the TV shows directory", default_value = "/home/zeus/Downloads/media/tv")]
    pub tv_dir: String,
    #[arg(long, help = "Path to the MPV IPC socket", default_value = "/tmp/mpvipc")]
    pub mpv_socket: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List {
        #[command(subcommand)]
        list_subcommand: ListSubcommand
    },
    Play {
        #[command(subcommand)]
        play_subcommand: PlaySubcommand
    },
}

#[derive(Subcommand)]
pub enum ListSubcommand {
    Recent,
    Library,
}

#[derive(Subcommand)]
pub enum PlaySubcommand {
    Last,
    Choose,
}

pub async fn handle_list_command(list_command: &ListSubcommand, index: &Library) -> Result<(), Box<dyn std::error::Error>> {
    match list_command {
        ListSubcommand::Library => {
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
        },
        ListSubcommand::Recent => {
            println!("Recent media listing not yet implemented.");
            Ok(())
        },
    }
}

pub async fn handle_play_command(play_command: &PlaySubcommand, index: &Library, mpv_socket: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut player = Player::init(mpv_socket).await?;

    match play_command {
        PlaySubcommand::Last => {
            println!("Playing last media playback command not yet implemented.");
            Ok(())
        },
        PlaySubcommand::Choose => {
            println!("Playing media playback command not yet implemented.");
            Ok(())
        },
        _ => {
            println!("Not implemented yet!");
            Ok(())
        },
    }
}