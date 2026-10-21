//! Prints the app database as the frontend would receive it.
//!
//! ```sh
//! cargo run --example dump -- [path-to-checkd.db]
//! ```
//!
//! Without an argument it reads the database from the platform app-data dir.

use std::path::PathBuf;

use checkd_lib::db::Db;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = match std::env::args().nth(1) {
        Some(arg) => PathBuf::from(arg),
        None => {
            let dir = dirs_data_dir()?;
            dir.join("checkd.db")
        }
    };

    println!("reading {}", path.display());
    let db = Db::open_path(&path)?;
    let data = db.load()?;
    println!("{}", serde_json::to_string_pretty(&data)?);
    Ok(())
}

/// Mirrors Tauri's app-data dir for the `com.kumar.checkd` identifier.
fn dirs_data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(dir).join("com.kumar.checkd"));
    }
    let home = std::env::var("HOME")?;
    Ok(PathBuf::from(home).join(".local/share/com.kumar.checkd"))
}
