mod indexer;
mod library;
mod mpv;
use clap::{Parser, Subcommand};
use indexer::Library;

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

fn list_library(index: &Library) {
    println!("Found {} Movies", &index.movies.len());
    println!("------------");
    for entry in &index.movies {
        println!("{}", entry.name)
    }
    println!("\n\n");
    println!("Found {} Shows", &index.shows.len());
    println!("------------");
    for tv_entry in &index.shows {
        println!("{}", tv_entry.name);
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let cli = Cli::parse();

    let index = indexer::index(
        String::from("/hdd/media/Movies"),
        String::from("/hdd/media/TV"),
    );

    match &cli.command {
        Commands::List { list_command } => match list_command {
            ListSubcommand::Library => list_library(&index),
            ListSubcommand::Recent => println!("Not implemented"),
        },
    };

    // let mut mpv = mpv::Player::init("/tmp/mpvipc").await?;

    // println!("{:?}", &index.movies[0].name);
    // mpv.play_file(&index.movies[0].path).await?;

    Ok(())
}
