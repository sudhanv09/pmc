use crate::cli::{App, Commands, handle_list_command, handle_play_command};
use clap::Parser;
mod cli;
mod db;
mod indexer;
mod library;
mod mpv;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let index = indexer::index(
        String::from("/home/zeus/Downloads/media/movies"),
        String::from("/home/zeus/Downloads/media/tv"),
    );
    
    let cli = App::parse();
    let result = match &cli.command {
        Commands::List { list_subcommand } => handle_list_command(list_subcommand, &index).await,
        Commands::Play { play_subcommand } => handle_play_command(play_subcommand, &index, &cli.mpv_socket).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        // exit with a non-zero code here
        std::process::exit(1);
    }

    Ok(())
}
