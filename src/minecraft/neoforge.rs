use std::cmp::Ordering;

use serde::Deserialize;
use tokio::process::Command;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::minecraft::java;
use crate::system::download::download_to_file;

const MAVEN_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
const MAVEN_BASE_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

#[derive(Debug, Deserialize)]
struct MavenMetadata {
    versioning: Versioning,
}

#[derive(Debug, Deserialize)]
struct Versioning {
    versions: Versions,
}

#[derive(Debug, Deserialize)]
struct Versions {
    version: Vec<String>,
}

pub async fn install(config: &AppConfig) -> AppResult<()> {
    crate::drive::files::require_structure(config).await?;
    let java = java::ensure(config).await?;
    let version = ensure_version(config).await?;
    let filename = format!("neoforge-{version}-installer.jar");
    let installer = config.root.join("downloads").join(&filename);
    let url = format!("{MAVEN_BASE_URL}/{version}/{filename}");
    let paths = config.paths();
    let launcher = if cfg!(windows) {
        paths.server.join("run.bat")
    } else {
        paths.server.join("run.sh")
    };
    if !launcher.is_file() || config.neoforge_version.as_deref() != Some(version.as_str()) {
        if !installer.is_file() {
            download_to_file(&url, &installer).await?;
        }
        let status = Command::new(java)
            .arg("-jar")
            .arg(&installer)
            .arg("--installServer")
            .current_dir(&paths.server)
            .status()
            .await
            .map_err(|error| AppError::Process(error.to_string()))?;
        if !status.success() {
            return Err(AppError::Process(format!(
                "NeoForge installer exited with {status}"
            )));
        }
    }
    persist_version(config, &version)?;
    Ok(())
}

pub async fn ensure_version(config: &AppConfig) -> AppResult<String> {
    if let Some(version) = config
        .neoforge_version
        .as_deref()
        .filter(|version| !version.is_empty())
    {
        return Ok(version.to_owned());
    }
    let version = latest_stable_for(config.minecraft_version.as_str()).await?;
    persist_version(config, &version)?;
    Ok(version)
}

async fn latest_stable_for(minecraft_version: &str) -> AppResult<String> {
    let family = match minecraft_version {
        "1.21.1" => "21.1",
        version => {
            return Err(AppError::Configuration(format!(
                "NeoForge version mapping is not available for Minecraft {version}"
            )));
        }
    };
    let response = reqwest::get(MAVEN_METADATA_URL)
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "GET {MAVEN_METADATA_URL} returned {}",
            response.status()
        )));
    }
    let metadata = response
        .text()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    let metadata: MavenMetadata = quick_xml::de::from_str(&metadata)
        .map_err(|error| AppError::Network(format!("invalid NeoForge metadata: {error}")))?;
    metadata
        .versioning
        .versions
        .version
        .into_iter()
        .filter(|version| version.starts_with(&format!("{family}.")) && !version.contains('-'))
        .filter(|version| parse_version(version).is_some())
        .max_by(|left, right| compare_versions(left, right))
        .ok_or_else(|| AppError::Network(format!("no stable NeoForge {family} release was found")))
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left = parse_version(left).unwrap_or_default();
    let right = parse_version(right).unwrap_or_default();
    left.cmp(&right)
}

fn parse_version(version: &str) -> Option<Vec<u32>> {
    version
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn persist_version(config: &AppConfig, version: &str) -> AppResult<()> {
    let mut updated = config.clone();
    updated.neoforge_version = Some(version.to_owned());
    updated.save()
}
