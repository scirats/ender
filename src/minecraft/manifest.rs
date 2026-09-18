use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

#[derive(Debug, Default, Deserialize)]
pub struct Manifest {
    pub minecraft_version: Option<String>,
    pub neoforge_version: Option<String>,
    #[serde(default)]
    pub mods: Vec<ModSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ModSpec {
    pub filename: String,
    pub sha256: String,
    pub side: ModSide,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModSide {
    Client,
    Server,
    Both,
}

pub fn validate(config: &AppConfig, side: ModSide, mods_root: Option<&Path>) -> AppResult<()> {
    if !config.paths().manifest.is_file() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&config.paths().manifest)
        .map_err(|error| AppError::io(config.paths().manifest, error))?;
    let manifest: Manifest = toml::from_str(&content)?;
    if let Some(version) = manifest.minecraft_version
        && version != config.minecraft_version
    {
        return Err(AppError::Configuration(format!(
            "manifest Minecraft version {version} does not match {}",
            config.minecraft_version
        )));
    }
    if let Some(version) = manifest.neoforge_version
        && !version.is_empty()
        && config.neoforge_version.as_deref() != Some(version.as_str())
    {
        return Err(AppError::Configuration(format!(
            "manifest NeoForge version {version} does not match the installed version"
        )));
    }
    let default_root = match side {
        ModSide::Client => config.paths().client.join("mods"),
        ModSide::Server | ModSide::Both => config.paths().server.join("mods"),
    };
    let root = mods_root.unwrap_or(&default_root);
    for spec in manifest.mods {
        if !applies_to(&spec.side, &side) {
            continue;
        }
        let path = root.join(&spec.filename);
        if !path.is_file() {
            return Err(AppError::Configuration(format!(
                "manifest mod is missing: {}",
                path.display()
            )));
        }
        let hash = sha256(&path)?;
        if !hash.eq_ignore_ascii_case(&spec.sha256) {
            return Err(AppError::Configuration(format!(
                "manifest hash mismatch for {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn applies_to(mod_side: &ModSide, requested_side: &ModSide) -> bool {
    mod_side == &ModSide::Both || mod_side == requested_side
}

fn sha256(path: &Path) -> AppResult<String> {
    let mut file = File::open(path).map_err(|error| AppError::io(path, error))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| AppError::io(path, error))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}
