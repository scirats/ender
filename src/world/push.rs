use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::server::lifecycle;
use crate::world::archive;

pub async fn run(config: &AppConfig) -> AppResult<()> {
    if lifecycle::running(config)? {
        return Err(AppError::Process(
            "stop the server before pushing the world".to_owned(),
        ));
    }
    if !config.paths().google_token.is_file() {
        return Err(AppError::Configuration(
            "Google Drive authentication is required before pushing the world".to_owned(),
        ));
    }
    let remote_config = crate::drive::files::ensure_structure(config).await?;
    if !crate::drive::files::can_write(&remote_config, &["backups"]).await? {
        return Err(AppError::Configuration(
            "the authenticated Google Drive user cannot write to the Maincraft folder".to_owned(),
        ));
    }
    let paths = config.paths();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::Archive(error.to_string()))?
        .as_nanos();
    let backup = paths.backups.join(format!("world-{timestamp}.tar.zst"));
    archive::create(&paths.world, &backup)?;
    let filename = backup
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::Archive("invalid backup filename".to_owned()))?;
    crate::drive::files::upload(&remote_config, &backup, &["backups", filename]).await?;
    println!("World pushed to Google Drive from {}", backup.display());
    Ok(())
}
