use crate::commands;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pmc")]
#[command(about = "simple CLI tool to manage your media", long_about = None)]
pub struct App {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List,
    Recent,
    Play,
    Resume,
    Sync,
}

impl Commands {
    pub async fn execute(&self, index: &crate::library::MediaLibrary) -> anyhow::Result<()> {
        match self {
            Commands::List => commands::list::execute(index).await,
            Commands::Recent => commands::recent::execute(index).await,
            Commands::Play => commands::play::execute(index).await,
            Commands::Resume => commands::resume::execute(index).await,
            Commands::Sync => commands::sync::execute(),
        }
    }
}