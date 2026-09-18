use std::fs;
use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

pub fn provision(config: &AppConfig, java_path: &Path) -> AppResult<PathBuf> {
    let instance = instance_path(config);
    fs::create_dir_all(instance.join("mods")).map_err(|error| AppError::io(&instance, error))?;
    fs::create_dir_all(instance.join("config")).map_err(|error| AppError::io(&instance, error))?;

    let instance_cfg = instance.join("instance.cfg");
    let instance_content = format!(
        "InstanceType=OneSix\nname=Maincraft\niconKey=default\nOverrideJava=true\nOverrideJavaLocation={}\n",
        java_path.display()
    );
    fs::write(&instance_cfg, instance_content)
        .map_err(|error| AppError::io(&instance_cfg, error))?;

    let pack = instance.join("mmc-pack.json");
    let neoforge_version = config
        .neoforge_version
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Configuration("NeoForge must be installed before the client".to_owned())
        })?;
    let content = serde_json::json!({
        "components": [
            {
                "cachedName": "Minecraft",
                "uid": "net.minecraft",
                "version": config.minecraft_version,
            },
            {
                "cachedName": "NeoForge",
                "uid": "net.neoforged",
                "version": neoforge_version,
            }
        ],
        "formatVersion": 1,
    });
    let content = serde_json::to_string_pretty(&content)
        .map_err(|error| AppError::Configuration(error.to_string()))?
        + "\n";
    fs::write(&pack, content).map_err(|error| AppError::io(&pack, error))?;
    Ok(instance)
}

pub fn instance_path(config: &AppConfig) -> PathBuf {
    config
        .prism_root
        .clone()
        .unwrap_or_else(default_prism_root)
        .join("instances")
        .join("maincraft")
}

fn default_prism_root() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support/PrismLauncher")
    } else if cfg!(target_os = "windows") {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("PrismLauncher")
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("PrismLauncher")
    }
}
