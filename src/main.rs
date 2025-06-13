use crate::cli::App;
use crate::library::load_or_configure_library;
use clap::Parser;

mod cli;
mod commands;
mod db;
mod indexer;
mod library;
mod mpv;
mod utils;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let index = load_or_configure_library().await?;

    db::Db::init("./pmc.db")
        .await
        .map_err(|e| {
            eprintln!("Database initialization failed: {}", e);
            std::process::exit(1);
        })
        .unwrap();

    let cli = App::parse();
    cli.command
        .execute(&index)
        .await
        .expect("Something went wrong");

    Ok(())
}
