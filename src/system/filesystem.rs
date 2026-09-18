use std::fs;
use std::path::Path;

use crate::error::{AppError, AppResult};

pub fn ensure_directory(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path).map_err(|error| AppError::io(path, error))
}
