use std::path::Path;

use google_drive3::yup_oauth2::{
    InstalledFlowAuthenticator, InstalledFlowReturnMethod, read_application_secret,
};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

pub async fn login(config: &AppConfig) -> AppResult<()> {
    let paths = config.paths();
    if !paths.google_client_secret.is_file() {
        return Err(AppError::Configuration(format!(
            "Google OAuth client secret not found: {}",
            paths.google_client_secret.display()
        )));
    }
    authenticate(&paths.google_client_secret, &paths.google_token).await?;
    let initialized = crate::drive::files::ensure_structure(config).await?;
    initialized.save()?;
    println!("Google Drive authentication completed");
    Ok(())
}

pub async fn logout(config: &AppConfig) -> AppResult<()> {
    let path = config.paths().google_token;
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|error| AppError::io(&path, error))?;
    }
    println!("Google Drive authentication removed");
    Ok(())
}

pub async fn authenticate(secret_path: &Path, token_path: &Path) -> AppResult<()> {
    let secret = read_application_secret(secret_path)
        .await
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    if let Some(parent) = token_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| AppError::io(parent, error))?;
    }
    InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk(token_path)
        .build()
        .await
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    Ok(())
}
