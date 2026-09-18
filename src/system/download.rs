use std::path::Path;

use futures_util::StreamExt;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};

pub async fn download_to_file(url: &str, destination: &Path) -> AppResult<()> {
    let response = reqwest::get(url)
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "GET {url} returned {}",
            response.status()
        )));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|error| AppError::io(parent, error))?;
    }
    let temporary = destination.with_extension("download");
    let mut file = fs::File::create(&temporary)
        .await
        .map_err(|error| AppError::io(&temporary, error))?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| AppError::Network(error.to_string()))?;
        file.write_all(&chunk)
            .await
            .map_err(|error| AppError::io(&temporary, error))?;
    }
    file.flush()
        .await
        .map_err(|error| AppError::io(&temporary, error))?;
    fs::rename(&temporary, destination)
        .await
        .map_err(|error| AppError::io(destination, error))
}
