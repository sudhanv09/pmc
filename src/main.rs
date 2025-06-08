use crate::cli::{App, Commands, handle_list_command, handle_play_command, handle_resume_command, handle_recent_command};
use clap::Parser;
mod cli;
mod db;
mod indexer;
mod library;
mod mpv;
mod utils;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let index = indexer::index(
        String::from("/home/zeus/Downloads/media/movies"),
        String::from("/home/zeus/Downloads/media/tv"),
    );

    db::Db::init("./pmc.db").await.map_err(|e| {
        eprintln!("Database initialization failed: {}", e);
        std::process::exit(1);
    }).unwrap();


    let cli = App::parse();
    let mpv = mpv::Player::init(&cli.mpv_socket).await.expect("Is mpv running? Run it with mpv --force-window --idle --input-ipc-server=<socketname>");

    let result = match &cli.command {
        Commands::List => handle_list_command(&index).await,
        Commands::Recent => handle_recent_command(&index).await,

        Commands::Play => handle_play_command(&index, mpv).await,
        Commands::Resume => handle_resume_command(&index, &mpv).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        // exit with a non-zero code here
        std::process::exit(1);
    }

    Ok(())
}
