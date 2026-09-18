use std::fs;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::server::lifecycle;
use crate::world::archive;

pub async fn run(config: &AppConfig) -> AppResult<()> {
    if lifecycle::running(config)? {
        return Err(AppError::Process(
            "stop the server before pulling the world".to_owned(),
        ));
    }
    let paths = config.paths();
    if !paths.google_token.is_file() {
        return Err(AppError::Configuration(
            "Google Drive authentication is required before pulling the world".to_owned(),
        ));
    }
    let remote_config = crate::drive::files::ensure_structure(config).await?;
    let archive_path = paths.root.join(".world-pull.tar.zst");
    crate::drive::files::download_latest_backup(&remote_config, &archive_path).await?;
    let staging = paths.root.join(".world-pull");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| AppError::io(&staging, error))?;
    }
    fs::create_dir_all(&staging).map_err(|error| AppError::io(&staging, error))?;
    archive::extract(&archive_path, &staging)?;
    let extracted = staging.join("world");
    if !extracted.is_dir() {
        return Err(AppError::Archive(
            "remote world archive has no world directory".to_owned(),
        ));
    }
    let previous = paths.root.join(".world-previous");
    if previous.exists() {
        fs::remove_dir_all(&previous).map_err(|error| AppError::io(&previous, error))?;
    }
    if paths.world.exists() {
        fs::rename(&paths.world, &previous).map_err(|error| AppError::io(&previous, error))?;
    }
    if let Err(error) = fs::rename(&extracted, &paths.world) {
        if previous.exists() {
            let _ = fs::rename(&previous, &paths.world);
        }
        return Err(AppError::io(&paths.world, error));
    }
    if previous.exists() {
        fs::remove_dir_all(&previous).map_err(|error| AppError::io(&previous, error))?;
    }
    fs::remove_dir_all(&staging).map_err(|error| AppError::io(&staging, error))?;
    fs::remove_file(&archive_path).map_err(|error| AppError::io(&archive_path, error))?;
    println!("World pulled from Google Drive");
    Ok(())
}
