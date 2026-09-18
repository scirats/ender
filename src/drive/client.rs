use google_drive3::DriveHub;
use google_drive3::hyper_rustls;
use google_drive3::hyper_util;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, read_application_secret};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

pub type Connector =
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;
pub type Hub = DriveHub<Connector>;

pub async fn connect(config: &AppConfig) -> AppResult<Hub> {
    let paths = config.paths();
    let secret = read_application_secret(&paths.google_client_secret)
        .await
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk(&paths.google_token)
        .build()
        .await
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|error| AppError::Configuration(error.to_string()))?
        .https_only()
        .enable_http2()
        .build();
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector.clone());
    Ok(DriveHub::new(client, auth))
}
