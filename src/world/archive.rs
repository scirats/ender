use std::fs::{self, File};
use std::path::{Component, Path};

use crate::error::{AppError, AppResult};

pub fn create(source: &Path, destination: &Path) -> AppResult<()> {
    if !source.is_dir() {
        return Err(AppError::Archive(format!(
            "world directory does not exist: {}",
            source.display()
        )));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    }
    let file = File::create(destination).map_err(|error| AppError::io(destination, error))?;
    let encoder = zstd::stream::write::Encoder::new(file, 3)
        .map_err(|error| AppError::Archive(error.to_string()))?;
    let mut archive = tar::Builder::new(encoder);
    archive
        .append_dir_all("world", source)
        .map_err(|error| AppError::Archive(error.to_string()))?;
    let encoder = archive
        .into_inner()
        .map_err(|error| AppError::Archive(error.to_string()))?;
    encoder
        .finish()
        .map_err(|error| AppError::Archive(error.to_string()))?;
    Ok(())
}

pub fn extract(source: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination).map_err(|error| AppError::io(destination, error))?;
    let file = File::open(source).map_err(|error| AppError::io(source, error))?;
    let decoder = zstd::stream::read::Decoder::new(file)
        .map_err(|error| AppError::Archive(error.to_string()))?;
    let mut archive = tar::Archive::new(decoder);
    for entry in archive
        .entries()
        .map_err(|error| AppError::Archive(error.to_string()))?
    {
        let mut entry = entry.map_err(|error| AppError::Archive(error.to_string()))?;
        let path = entry
            .path()
            .map_err(|error| AppError::Archive(error.to_string()))?;
        if path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(AppError::Archive(
                "archive contains an unsafe path".to_owned(),
            ));
        }
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink()
            || entry_type.is_hard_link()
            || !entry_type.is_file() && !entry_type.is_dir()
        {
            return Err(AppError::Archive(
                "archive contains an unsupported entry type".to_owned(),
            ));
        }
        entry
            .unpack_in(destination)
            .map_err(|error| AppError::Archive(error.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{create, extract};

    #[test]
    fn creates_and_extracts_a_world_archive() {
        let directory = tempdir().expect("temporary directory should be created");
        let source = directory.path().join("world");
        let archive = directory.path().join("world.tar.zst");
        let destination = directory.path().join("destination");
        fs::create_dir_all(&source).expect("world should be created");
        fs::write(source.join("level.dat"), "world").expect("world data should be written");

        create(&source, &archive).expect("archive should be created");
        extract(&archive, &destination).expect("archive should be extracted");

        assert_eq!(
            fs::read_to_string(destination.join("world/level.dat")).unwrap(),
            "world"
        );
    }
}
