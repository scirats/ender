use std::fs;

use serde::Serialize;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::system::filesystem::ensure_directory;
use crate::system::paths::{client_config, client_mods, server_config, server_mods};

#[derive(Serialize)]
struct LocalManifest<'a> {
    minecraft_version: &'a str,
    neoforge_version: Option<&'a str>,
    java_version: u8,
    world_name: &'a str,
}

pub async fn run(config: &AppConfig) -> AppResult<()> {
    let paths = config.paths();
    let server_mods_path = server_mods(&paths.server);
    let server_config_path = server_config(&paths.server);
    let client_mods_path = client_mods(&paths.client);
    let client_config_path = client_config(&paths.client);
    let directories = [
        paths.root.as_path(),
        paths.server.as_path(),
        paths.client.as_path(),
        paths.world.as_path(),
        paths.backups.as_path(),
        paths.java.as_path(),
        server_mods_path.as_path(),
        server_config_path.as_path(),
        client_mods_path.as_path(),
        client_config_path.as_path(),
    ];

    for directory in directories {
        ensure_directory(directory)?;
    }

    if !paths.manifest.exists() {
        let manifest = toml::to_string_pretty(&LocalManifest {
            minecraft_version: &config.minecraft_version,
            neoforge_version: config.neoforge_version.as_deref(),
            java_version: config.java_version,
            world_name: &config.world_name,
        })?;
        fs::write(&paths.manifest, manifest)
            .map_err(|error| AppError::io(&paths.manifest, error))?;
    }

    config.save()?;
    if paths.google_client_secret.is_file() && paths.google_token.is_file() {
        let remote_config = crate::drive::files::ensure_structure(config).await?;
        let remote_manifest = ["manifest.toml"];
        if crate::drive::files::exists(&remote_config, &remote_manifest).await? {
            let downloaded = paths.root.join(".manifest.remote");
            crate::drive::files::download(&remote_config, &remote_manifest, &downloaded).await?;
            std::fs::rename(&downloaded, &paths.manifest)
                .map_err(|error| AppError::io(&paths.manifest, error))?;
        } else {
            crate::drive::files::upload(&remote_config, &paths.manifest, &remote_manifest).await?;
        }
        crate::drive::files::sync_folder(
            &remote_config,
            &["server", "mods"],
            &paths.server.join("mods"),
        )
        .await?;
        crate::drive::files::sync_folder(
            &remote_config,
            &["server", "config"],
            &paths.server.join("config"),
        )
        .await?;
        crate::drive::files::sync_folder(
            &remote_config,
            &["client", "mods"],
            &paths.client.join("mods"),
        )
        .await?;
        crate::drive::files::sync_folder(
            &remote_config,
            &["client", "config"],
            &paths.client.join("config"),
        )
        .await?;
        if !paths.world.join("level.dat").is_file()
            && crate::drive::files::latest_backup_exists(&remote_config).await?
        {
            crate::world::pull::run(&remote_config).await?;
        }
    }

    println!("Maincraft is initialized at {}", config.root.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::run;
    use crate::config::AppConfig;

    #[tokio::test]
    async fn initializes_the_expected_structure() {
        let directory = tempdir().expect("temporary directory should be created");
        let config = AppConfig::new(directory.path().to_owned());

        run(&config).await.expect("initialization should succeed");

        let paths = config.paths();
        assert!(paths.server.is_dir());
        assert!(paths.client.is_dir());
        assert!(paths.world.is_dir());
        assert!(paths.backups.is_dir());
        assert!(paths.manifest.is_file());
    }

    #[tokio::test]
    async fn does_not_replace_an_existing_manifest() {
        let directory = tempdir().expect("temporary directory should be created");
        let config = AppConfig::new(directory.path().to_owned());
        fs::create_dir_all(&config.root).expect("root should be created");
        let manifest = config.paths().manifest;
        fs::write(&manifest, "existing = true").expect("manifest should be written");

        run(&config).await.expect("initialization should succeed");

        assert_eq!(fs::read_to_string(manifest).unwrap(), "existing = true");
    }
}
