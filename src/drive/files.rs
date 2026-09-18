use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufReader, Cursor};
use std::path::{Component, Path, PathBuf};

use google_drive3::api::File as DriveFile;
use http_body_util::BodyExt;
use mime::Mime;

use crate::config::AppConfig;
use crate::drive::client;
use crate::error::{AppError, AppResult};

const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";

pub async fn ensure_structure(config: &AppConfig) -> AppResult<AppConfig> {
    let mut config = config.clone();
    let hub = client::connect(&config).await?;
    let root = match config.drive_folder_id.clone() {
        Some(id) if is_folder(&hub, &id).await? => id,
        _ => find_or_create_folder(&hub, "ender", None).await?,
    };
    config.drive_folder_id = Some(root.clone());
    let server = find_or_create_folder(&hub, "server", Some(&root)).await?;
    let client = find_or_create_folder(&hub, "client", Some(&root)).await?;
    let backups = find_or_create_folder(&hub, "backups", Some(&root)).await?;
    let server_mods = find_or_create_folder(&hub, "mods", Some(&server)).await?;
    let server_config = find_or_create_folder(&hub, "config", Some(&server)).await?;
    let client_mods = find_or_create_folder(&hub, "mods", Some(&client)).await?;
    let client_config = find_or_create_folder(&hub, "config", Some(&client)).await?;
    config.drive_server_folder_id = Some(server);
    config.drive_client_folder_id = Some(client);
    config.drive_backups_folder_id = Some(backups);
    config.drive_server_mods_folder_id = Some(server_mods);
    config.drive_server_config_folder_id = Some(server_config);
    config.drive_client_mods_folder_id = Some(client_mods);
    config.drive_client_config_folder_id = Some(client_config);
    config.save()?;
    Ok(config)
}

pub async fn require_structure(config: &AppConfig) -> AppResult<AppConfig> {
    let required = [
        ("Drive root", config.drive_folder_id.as_deref()),
        (
            "Drive server folder",
            config.drive_server_folder_id.as_deref(),
        ),
        (
            "Drive client folder",
            config.drive_client_folder_id.as_deref(),
        ),
        (
            "Drive backups folder",
            config.drive_backups_folder_id.as_deref(),
        ),
        (
            "Drive server mods folder",
            config.drive_server_mods_folder_id.as_deref(),
        ),
        (
            "Drive server config folder",
            config.drive_server_config_folder_id.as_deref(),
        ),
        (
            "Drive client mods folder",
            config.drive_client_mods_folder_id.as_deref(),
        ),
        (
            "Drive client config folder",
            config.drive_client_config_folder_id.as_deref(),
        ),
    ];
    if let Some((name, None)) = required.iter().find(|(_, id)| id.is_none()) {
        return Err(AppError::Configuration(format!(
            "{name} is not configured; run init first"
        )));
    }
    if !config.paths().server.is_dir() || !config.paths().client.is_dir() {
        return Err(AppError::Configuration(
            "local structure is not initialized; run init first".to_owned(),
        ));
    }
    let hub = client::connect(config).await?;
    for (name, id) in required {
        let Some(id) = id else {
            return Err(AppError::Configuration(format!(
                "{name} is not configured; run init first"
            )));
        };
        if !is_folder(&hub, id).await? {
            return Err(AppError::Configuration(format!(
                "{name} is no longer available; run init again"
            )));
        }
    }
    Ok(config.clone())
}

async fn is_folder(hub: &client::Hub, id: &str) -> AppResult<bool> {
    let result = hub
        .files()
        .get(id)
        .add_scope(DRIVE_SCOPE)
        .supports_all_drives(true)
        .param("fields", "id,mimeType,trashed")
        .doit()
        .await;
    match result {
        Ok((_, file)) => {
            Ok(file.mime_type.as_deref() == Some(FOLDER_MIME) && !file.trashed.unwrap_or(false))
        }
        Err(error) if error.to_string().contains("404") => Ok(false),
        Err(error) => Err(AppError::Network(error.to_string())),
    }
}

pub async fn upload(config: &AppConfig, local: &Path, remote_path: &[&str]) -> AppResult<()> {
    let hub = client::connect(config).await?;
    let parent = ensure_parent_path(&hub, config, remote_path).await?;
    let name = remote_path
        .last()
        .ok_or_else(|| AppError::Configuration("remote path cannot be empty".to_owned()))?;
    let existing = find_file(&hub, name, Some(&parent), false).await?;
    let file = File::open(local).map_err(|error| AppError::io(local, error))?;
    let metadata = DriveFile {
        name: Some((*name).to_owned()),
        parents: existing.is_none().then(|| vec![parent.clone()]),
        ..Default::default()
    };
    let mime: Mime = "application/octet-stream"
        .parse()
        .map_err(|error| AppError::Configuration(format!("invalid MIME type: {error}")))?;
    let result = match existing {
        Some(existing) => {
            hub.files()
                .update(metadata, &existing)
                .add_scope(DRIVE_SCOPE)
                .supports_all_drives(true)
                .upload(BufReader::new(file), mime)
                .await
        }
        None => {
            hub.files()
                .create(metadata)
                .add_scope(DRIVE_SCOPE)
                .supports_all_drives(true)
                .upload(BufReader::new(file), mime)
                .await
        }
    };
    result.map_err(|error| AppError::Network(error.to_string()))?;
    Ok(())
}

pub async fn download(config: &AppConfig, remote_path: &[&str], local: &Path) -> AppResult<()> {
    let hub = client::connect(config).await?;
    let parent = ensure_parent_path(&hub, config, remote_path).await?;
    let name = remote_path
        .last()
        .ok_or_else(|| AppError::Configuration("remote path cannot be empty".to_owned()))?;
    let file_id = find_file(&hub, name, Some(&parent), false)
        .await?
        .ok_or_else(|| AppError::Network(format!("Drive file not found: {name}")))?;
    download_file_to_path(&hub, &file_id, local).await
}

pub async fn can_write(config: &AppConfig, remote_path: &[&str]) -> AppResult<bool> {
    let hub = client::connect(config).await?;
    let folder_id = resolve_folder_path(&hub, config, remote_path).await?;
    let (_, file) = hub
        .files()
        .get(&folder_id)
        .add_scope(DRIVE_SCOPE)
        .supports_all_drives(true)
        .param(
            "fields",
            "id,capabilities(canAddChildren,canEdit,canModifyContent)",
        )
        .doit()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    Ok(file
        .capabilities
        .and_then(|capabilities| capabilities.can_add_children.or(capabilities.can_edit))
        .unwrap_or(false))
}

pub async fn exists(config: &AppConfig, remote_path: &[&str]) -> AppResult<bool> {
    let hub = client::connect(config).await?;
    let parent = resolve_folder_path(
        &hub,
        config,
        &remote_path[..remote_path.len().saturating_sub(1)],
    )
    .await?;
    let name = remote_path
        .last()
        .ok_or_else(|| AppError::Configuration("remote path cannot be empty".to_owned()))?;
    Ok(find_file(&hub, name, Some(&parent), false).await?.is_some())
}

pub async fn latest_backup_exists(config: &AppConfig) -> AppResult<bool> {
    let hub = client::connect(config).await?;
    Ok(latest_backup_with_hub(&hub, config).await?.is_some())
}

pub async fn download_latest_backup(config: &AppConfig, local: &Path) -> AppResult<()> {
    let hub = client::connect(config).await?;
    let file_id = latest_backup_with_hub(&hub, config).await?.ok_or_else(|| {
        AppError::Network("no world backup is available in Google Drive".to_owned())
    })?;
    download_file_to_path(&hub, &file_id, local).await
}

pub async fn sync_folder(
    config: &AppConfig,
    remote_path: &[&str],
    local_directory: &Path,
) -> AppResult<()> {
    let hub = client::connect(config).await?;
    let folder_id = resolve_folder_path(&hub, config, remote_path).await?;
    let remote_files = list_files(
        &hub,
        &format!("'{folder_id}' in parents and trashed = false"),
    )
    .await?;
    tokio::fs::create_dir_all(local_directory)
        .await
        .map_err(|error| AppError::io(local_directory, error))?;
    let mut remote_names = HashSet::new();
    for file in remote_files {
        let Some(id) = file.id else { continue };
        let Some(name) = file.name else { continue };
        remote_names.insert(name.clone());
        if file.mime_type.as_deref() == Some(FOLDER_MIME) {
            continue;
        }
        let destination = safe_child_path(local_directory, &name)?;
        download_file_to_path(&hub, &id, &destination).await?;
    }
    for entry in
        fs::read_dir(local_directory).map_err(|error| AppError::io(local_directory, error))?
    {
        let entry = entry.map_err(|error| AppError::io(local_directory, error))?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| !remote_names.contains(name))
        {
            fs::remove_file(&path).map_err(|error| AppError::io(&path, error))?;
        }
    }
    Ok(())
}

pub async fn push_folder(
    config: &AppConfig,
    remote_path: &[&str],
    local_directory: &Path,
) -> AppResult<()> {
    if !local_directory.is_dir() {
        return Err(AppError::Configuration(format!(
            "local directory does not exist: {}",
            local_directory.display()
        )));
    }
    let hub = client::connect(config).await?;
    let folder_id = resolve_folder_path(&hub, config, remote_path).await?;
    let remote_files = list_files(
        &hub,
        &format!("'{folder_id}' in parents and trashed = false"),
    )
    .await?;
    let mut local_names = HashSet::new();
    for entry in
        fs::read_dir(local_directory).map_err(|error| AppError::io(local_directory, error))?
    {
        let entry = entry.map_err(|error| AppError::io(local_directory, error))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
        if !metadata.file_type().is_file() {
            continue;
        }
        let name = safe_local_name(&path)?;
        local_names.insert(name.to_owned());
        upload_file(
            &hub,
            &path,
            name,
            &folder_id,
            find_remote_file(&remote_files, name),
        )
        .await?;
    }
    for file in remote_files {
        let Some(id) = file.id else { continue };
        let Some(name) = file.name else { continue };
        if file.mime_type.as_deref() != Some(FOLDER_MIME) && !local_names.contains(&name) {
            hub.files()
                .delete(&id)
                .add_scope(DRIVE_SCOPE)
                .supports_all_drives(true)
                .doit()
                .await
                .map_err(|error| AppError::Network(error.to_string()))?;
        }
    }
    println!("Pushed {} to Google Drive", remote_path.join("/"));
    Ok(())
}

async fn ensure_parent_path(
    hub: &client::Hub,
    config: &AppConfig,
    remote_path: &[&str],
) -> AppResult<String> {
    let root = config
        .drive_folder_id
        .as_deref()
        .ok_or_else(|| AppError::Configuration("Drive folder is not configured".to_owned()))?;
    let mut parent = root.to_owned();
    for component in remote_path.iter().take(remote_path.len().saturating_sub(1)) {
        parent = find_or_create_folder(hub, component, Some(&parent)).await?;
    }
    Ok(parent)
}

fn find_remote_file<'a>(files: &'a [DriveFile], name: &str) -> Option<&'a str> {
    files.iter().find_map(|file| {
        (file.name.as_deref() == Some(name) && file.mime_type.as_deref() != Some(FOLDER_MIME))
            .then_some(file.id.as_deref())
            .flatten()
    })
}

fn safe_local_name(path: &Path) -> AppResult<&str> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::Configuration(format!("invalid local filename: {}", path.display()))
        })?;
    if name.is_empty() || name == "." || name == ".." {
        return Err(AppError::Configuration(format!(
            "invalid local filename: {name}"
        )));
    }
    Ok(name)
}

async fn upload_file(
    hub: &client::Hub,
    local: &Path,
    name: &str,
    parent: &str,
    existing: Option<&str>,
) -> AppResult<()> {
    let file = File::open(local).map_err(|error| AppError::io(local, error))?;
    let metadata = DriveFile {
        name: Some(name.to_owned()),
        parents: existing.is_none().then(|| vec![parent.to_owned()]),
        ..Default::default()
    };
    let mime: Mime = "application/octet-stream"
        .parse()
        .map_err(|error| AppError::Configuration(format!("invalid MIME type: {error}")))?;
    let result = match existing {
        Some(id) => {
            hub.files()
                .update(metadata, id)
                .add_scope(DRIVE_SCOPE)
                .supports_all_drives(true)
                .upload(BufReader::new(file), mime)
                .await
        }
        None => {
            hub.files()
                .create(metadata)
                .add_scope(DRIVE_SCOPE)
                .supports_all_drives(true)
                .upload(BufReader::new(file), mime)
                .await
        }
    };
    result
        .map(|_| ())
        .map_err(|error| AppError::Network(error.to_string()))
}

async fn resolve_folder_path(
    hub: &client::Hub,
    config: &AppConfig,
    remote_path: &[&str],
) -> AppResult<String> {
    let mut parent = config
        .drive_folder_id
        .as_deref()
        .ok_or_else(|| AppError::Configuration("Drive folder is not configured".to_owned()))?
        .to_owned();
    for component in remote_path {
        if let Some(id) = known_folder_id(config, component, &parent) {
            parent = id;
        } else {
            parent = find_folder_with_retry(hub, component, &parent).await?;
        }
    }
    Ok(parent)
}

fn known_folder_id(config: &AppConfig, name: &str, parent: &str) -> Option<String> {
    if parent == config.drive_folder_id.as_deref()? {
        return match name {
            "server" => config.drive_server_folder_id.clone(),
            "client" => config.drive_client_folder_id.clone(),
            "backups" => config.drive_backups_folder_id.clone(),
            _ => None,
        };
    }
    if parent == config.drive_server_folder_id.as_deref()? {
        return match name {
            "mods" => config.drive_server_mods_folder_id.clone(),
            "config" => config.drive_server_config_folder_id.clone(),
            _ => None,
        };
    }
    if parent == config.drive_client_folder_id.as_deref()? {
        return match name {
            "mods" => config.drive_client_mods_folder_id.clone(),
            "config" => config.drive_client_config_folder_id.clone(),
            _ => None,
        };
    }
    None
}

async fn find_folder_with_retry(hub: &client::Hub, name: &str, parent: &str) -> AppResult<String> {
    for attempt in 0..3 {
        if let Some(id) = find_file(hub, name, Some(parent), true).await? {
            return Ok(id);
        }
        if attempt < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    Err(AppError::Network(format!("Drive folder not found: {name}")))
}

async fn latest_backup_with_hub(
    hub: &client::Hub,
    config: &AppConfig,
) -> AppResult<Option<String>> {
    let folder_id = resolve_folder_path(hub, config, &["backups"]).await?;
    let files = list_files(
        hub,
        &format!("'{folder_id}' in parents and trashed = false"),
    )
    .await?;
    Ok(files
        .into_iter()
        .filter(|file| {
            file.mime_type.as_deref() != Some(FOLDER_MIME)
                && file
                    .name
                    .as_deref()
                    .is_some_and(|name| name.starts_with("world-") && name.ends_with(".tar.zst"))
        })
        .filter_map(|file| Some((file.name?, file.id?)))
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, id)| id))
}

async fn download_file_to_path(hub: &client::Hub, id: &str, local: &Path) -> AppResult<()> {
    let (response, _) = hub
        .files()
        .get(id)
        .add_scope(DRIVE_SCOPE)
        .supports_all_drives(true)
        .param("alt", "media")
        .doit()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?
        .to_bytes();
    if let Some(parent) = local.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| AppError::io(parent, error))?;
    }
    if let Ok(metadata) = tokio::fs::symlink_metadata(local).await
        && metadata.file_type().is_symlink()
    {
        return Err(AppError::io(
            local,
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "refusing to replace a symbolic link",
            ),
        ));
    }
    let temporary = local.with_file_name(format!(
        ".{}.download",
        local
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file")
    ));
    tokio::fs::write(&temporary, body)
        .await
        .map_err(|error| AppError::io(&temporary, error))?;
    tokio::fs::rename(&temporary, local)
        .await
        .map_err(|error| AppError::io(local, error))
}

async fn find_or_create_folder(
    hub: &client::Hub,
    name: &str,
    parent: Option<&str>,
) -> AppResult<String> {
    if let Some(file) = find_file(hub, name, parent, true).await? {
        return Ok(file);
    }
    create_folder(hub, name, parent).await
}

async fn create_folder(hub: &client::Hub, name: &str, parent: Option<&str>) -> AppResult<String> {
    let mut metadata = DriveFile {
        name: Some(name.to_owned()),
        mime_type: Some(FOLDER_MIME.to_owned()),
        ..Default::default()
    };
    if let Some(parent) = parent {
        metadata.parents = Some(vec![parent.to_owned()]);
    }
    let (_, file) = hub
        .files()
        .create(metadata)
        .add_scope(DRIVE_SCOPE)
        .supports_all_drives(true)
        .param("fields", "id")
        .upload(
            Cursor::new(Vec::new()),
            "application/octet-stream"
                .parse()
                .map_err(|error| AppError::Configuration(format!("invalid MIME type: {error}")))?,
        )
        .await
        .map_err(|error| AppError::Network(error.to_string()))?;
    file.id
        .ok_or_else(|| AppError::Network(format!("Drive did not return an ID for folder {name}")))
}

async fn find_file(
    hub: &client::Hub,
    name: &str,
    parent: Option<&str>,
    folder: bool,
) -> AppResult<Option<String>> {
    let mut query = format!("name = '{}' and trashed = false", name.replace('\'', "\\'"));
    if let Some(parent) = parent {
        query.push_str(&format!(" and '{parent}' in parents"));
    }
    if folder {
        query.push_str(&format!(" and mimeType = '{FOLDER_MIME}'"));
    }
    Ok(list_files(hub, &query)
        .await?
        .into_iter()
        .find_map(|file| file.id))
}

async fn list_files(hub: &client::Hub, query: &str) -> AppResult<Vec<DriveFile>> {
    let mut files = Vec::new();
    let mut page_token = None;
    loop {
        let mut request = hub
            .files()
            .list()
            .add_scope(DRIVE_SCOPE)
            .q(query)
            .spaces("drive")
            .param("fields", "nextPageToken,files(id,name,mimeType)")
            .page_size(1000)
            .supports_all_drives(true)
            .include_items_from_all_drives(true);
        if let Some(token) = page_token.as_deref() {
            request = request.page_token(token);
        }
        let (_, page) = request
            .doit()
            .await
            .map_err(|error| AppError::Network(error.to_string()))?;
        files.extend(page.files.unwrap_or_default());
        page_token = page.next_page_token;
        if page_token.is_none() {
            return Ok(files);
        }
    }
}

fn safe_child_path(root: &Path, name: &str) -> AppResult<PathBuf> {
    let path = Path::new(name);
    if name.is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path.components().count() != 1
    {
        return Err(AppError::Configuration(format!(
            "unsafe Drive filename: {name}"
        )));
    }
    let destination = root.join(name);
    Ok(destination)
}
