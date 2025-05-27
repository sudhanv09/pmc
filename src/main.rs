mod indexer;
mod library;
mod mpv;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pmc")]
#[command(about = "simple CLI tool to manage your media", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List {
        #[command(subcommand)]
        list_command: ListSubcommand,
    },
}

#[derive(Subcommand)]
pub enum ListSubcommand {
    Recent,
    Library,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let index = indexer::index(
        String::from("/hdd/media/Movies"),
        String::from("/hdd/media/TV"),
    );

    let mut mpv = mpv::Player::init("/tmp/mpvipc").await?;

    println!("{:?}", &index.movies[0].name);
    mpv.play_file(&index.movies[0].path).await?;

    Ok(())
}
