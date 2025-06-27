use crate::commands;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pmc")]
#[command(about = "simple CLI tool to manage your media", long_about = None)]
pub struct App {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_alias = "ls", about = "list indexed media files")]
    List,
    #[command(about = "list 5 most recently viewed items")]
    Recent,
    #[command(about = "play item")]
    Play(PlayArgs),
    #[command(about = "resume last item")]
    Resume,
    #[command(about = "reindex media files")]
    Sync,
    #[command(visible_alias = "org", about = "list indexed media files")]
    Organize,
}

#[derive(Args)]
pub struct PlayArgs {
    /// Play a movie
    #[arg(short = 'm', long = "movie", conflicts_with = "tv")]
    pub movie: bool,

    /// Play a TV show
    #[arg(short = 't', long = "tv", conflicts_with = "movie")]
    pub tv: bool,
}


impl Commands {
    pub async fn execute(&self, index: &crate::library::MediaLibrary) -> anyhow::Result<()> {
        match self {
            Commands::List => commands::list::execute(index).await,
            Commands::Recent => commands::recent::execute(index).await,
            Commands::Play(args) => commands::play::execute(index, args).await,
            Commands::Resume => commands::resume::execute(index).await,
            Commands::Sync => commands::sync::execute(),
            Commands::Organize => todo!(),
        }
    }
}
