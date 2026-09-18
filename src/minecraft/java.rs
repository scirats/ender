use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::process::Command;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::system::download::download_to_file;

#[derive(Debug, Deserialize)]
struct AdoptiumAsset {
    binary: AdoptiumBinary,
}

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    link: String,
    name: String,
}

pub async fn ensure(config: &AppConfig) -> AppResult<PathBuf> {
    if let Some(path) = bundled_java(&config.paths().java) {
        return Ok(path);
    }
    if let Some(path) = system_java(config.java_version).await {
        return Ok(path);
    }
    install(config).await
}

pub async fn install(config: &AppConfig) -> AppResult<PathBuf> {
    let paths = config.paths();
    let package = fetch_package(config.java_version).await?;
    let archive = paths.root.join("downloads").join(&package.name);
    download_to_file(&package.link, &archive).await?;
    let java_root = paths.java.clone();
    tokio::task::spawn_blocking(move || extract_archive(&archive, &java_root))
        .await
        .map_err(|error| AppError::Archive(error.to_string()))??;
    bundled_java(&paths.java).ok_or_else(|| {
        AppError::Archive("the downloaded archive does not contain a Java executable".to_owned())
    })
}

async fn fetch_package(version: u8) -> AppResult<AdoptiumPackage> {
    let url = format!(
        "https://api.adoptium.net/v3/assets/latest/{version}/hotspot?architecture={}&image_type=jdk&os={}&vendor=eclipse",
        architecture(),
        operating_system()
    );
    let response = reqwest::get(&url)
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "GET {url} returned {}",
            response.status()
        )));
    }
    let assets: Vec<AdoptiumAsset> = response
        .json()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    assets
        .into_iter()
        .next()
        .map(|asset| asset.binary.package)
        .ok_or_else(|| AppError::Network("Adoptium returned no matching Java package".to_owned()))
}

async fn system_java(required_version: u8) -> Option<PathBuf> {
    if let Ok(java_home) = env::var("JAVA_HOME") {
        let path = java_home_path(Path::new(&java_home));
        if path.is_file() && java_matches(&path, required_version).await {
            return Some(path);
        }
    }
    let output = Command::new(java_command())
        .arg("-version")
        .output()
        .await
        .ok()?;
    if output.status.success()
        && java_output_matches(&output.stderr, &output.stdout, required_version)
    {
        Some(resolve_java_command())
    } else {
        None
    }
}

async fn java_matches(path: &Path, required_version: u8) -> bool {
    Command::new(path)
        .arg("-version")
        .output()
        .await
        .map(|output| {
            output.status.success()
                && java_output_matches(&output.stderr, &output.stdout, required_version)
        })
        .unwrap_or(false)
}

fn java_output_matches(stderr: &[u8], stdout: &[u8], required_version: u8) -> bool {
    let output = [stderr, stdout].concat();
    let output = String::from_utf8_lossy(&output);
    let Some(version) = output.split('"').nth(1) else {
        return false;
    };
    let mut components = version.split('.');
    let major = components.next().and_then(|value| value.parse::<u8>().ok());
    let major = match major {
        Some(1) => components.next().and_then(|value| value.parse::<u8>().ok()),
        value => value,
    };
    major.is_some_and(|major| major == required_version)
}

fn bundled_java(root: &Path) -> Option<PathBuf> {
    let executable = java_executable_name();
    let direct = root.join("bin").join(executable);
    if direct.is_file() {
        return Some(direct);
    }
    find_file(root, executable)
}

fn extract_archive(archive: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination).map_err(|error| AppError::io(destination, error))?;
    if archive.extension().and_then(|value| value.to_str()) == Some("zip") {
        let file = File::open(archive).map_err(|error| AppError::io(archive, error))?;
        let mut zip =
            zip::ZipArchive::new(file).map_err(|error| AppError::Archive(error.to_string()))?;
        zip.extract(destination)
            .map_err(|error| AppError::Archive(error.to_string()))?;
        return Ok(());
    }
    let file = File::open(archive).map_err(|error| AppError::io(archive, error))?;
    let decoder = flate2::read::GzDecoder::new(file);
    tar::Archive::new(decoder)
        .unpack(destination)
        .map_err(|error| AppError::Archive(error.to_string()))
}

fn find_file(root: &Path, filename: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|value| value.to_str()) == Some(filename) {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_file(&path, filename)
        {
            return Some(found);
        }
    }
    None
}

fn java_home_path(root: &Path) -> PathBuf {
    root.join("bin").join(java_executable_name())
}

fn java_executable_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

fn java_command() -> &'static str {
    java_executable_name()
}

fn resolve_java_command() -> PathBuf {
    let command = java_command();
    let directories = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    directories
        .into_iter()
        .map(|directory| directory.join(command))
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(command))
}

fn operating_system() -> &'static str {
    match env::consts::OS {
        "macos" => "mac",
        "windows" => "windows",
        "linux" => "linux",
        value => value,
    }
}

fn architecture() -> &'static str {
    match env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        "x86" => "x32",
        value => value,
    }
}
