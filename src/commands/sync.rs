use std::path::Path;
use anyhow::{bail, Result};
use std::fs;
use crate::indexer;
use crate::library::MediaLibrary;

pub fn execute() -> Result<()> {
    let index_path = "./index.json";

    if !Path::new(index_path).exists() {
        bail!("index.json not found. Run the app to configure your library first.");
    }
    
    let data = fs::read_to_string(index_path)?;
    let current_library: MediaLibrary = serde_json::from_str(&data)?;

    // Re-index
    let updated_index = indexer::index(
        current_library.movie_dir.clone(),
        current_library.tv_dir.clone(),
    );

    let updated_library = MediaLibrary::new(
        current_library.movie_dir,
        current_library.tv_dir,
        updated_index,
    );

    let serialized = serde_json::to_string_pretty(&updated_library)?;
    fs::write(index_path, serialized)?;

    println!("Library successfully re-indexed and saved.");
    
    Ok(())
}