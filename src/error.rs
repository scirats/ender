use std::io;
use std::path::PathBuf;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O operation failed for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("network request failed: {0}")]
    Network(String),
    #[error("external process failed: {0}")]
    Process(String),
    #[error("archive operation failed: {0}")]
    Archive(String),
}

impl AppError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(error: toml::ser::Error) -> Self {
        Self::Configuration(error.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(error: toml::de::Error) -> Self {
        Self::Configuration(error.to_string())
    }
}
