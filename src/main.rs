use crate::cli::App;
use crate::library::load_or_configure_library;
use clap::Parser;
use anyhow::{Result, Context};

mod cli;
mod commands;
mod db;
mod indexer;
mod library;
mod mpv;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let index = load_or_configure_library().await.context("Failed to load library")?;

    db::Db::init("./pmc.db")
        .await
        .context("Failed to initialize database")?;

    let cli = App::parse();
    cli.command
        .execute(&index)
        .await
        .context("Something went wrong while executing command")?;

    Ok(())
}
