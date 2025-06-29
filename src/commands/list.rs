use crate::library::MediaLibrary;
use anyhow::Result;
use colored::Colorize;

pub async fn execute(index: &MediaLibrary) -> Result<()> {
    println!("{}", "Movies".green());
    println!("------");
    for item in &index.library.movies {
        println!("{}", &item.name);
    }
    println!("\n");

    println!("{}", "Shows".green());
    println!("------");
    for item in &index.library.shows {
        println!("{} - {} seasons", &item.name, &item.seasons.len());
    }
    Ok(())
}