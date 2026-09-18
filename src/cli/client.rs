use crate::config::AppConfig;
use crate::error::AppResult;

pub async fn run(config: &AppConfig, command: super::ClientAction) -> AppResult<()> {
    match command {
        super::ClientAction::Install => {
            let mut remote_config = crate::drive::files::require_structure(config).await?;
            let neoforge_version =
                crate::minecraft::neoforge::ensure_version(&remote_config).await?;
            remote_config.neoforge_version = Some(neoforge_version);
            let java_path = crate::minecraft::java::ensure(&remote_config).await?;
            let instance = crate::prism::instance::provision(&remote_config, &java_path)?;
            println!("Prism Launcher instance prepared at {}", instance.display());
            Ok(())
        }
        super::ClientAction::Push => {
            let mut remote_config = crate::drive::files::require_structure(config).await?;
            let neoforge_version =
                crate::minecraft::neoforge::ensure_version(&remote_config).await?;
            remote_config.neoforge_version = Some(neoforge_version);
            let java_path = crate::minecraft::java::ensure(&remote_config).await?;
            let instance = crate::prism::instance::provision(&remote_config, &java_path)?;
            crate::minecraft::manifest::validate(
                &remote_config,
                crate::minecraft::manifest::ModSide::Client,
                Some(&instance.join("mods")),
            )?;
            crate::drive::files::push_folder(
                &remote_config,
                &["client", "mods"],
                &instance.join("mods"),
            )
            .await?;
            crate::drive::files::push_folder(
                &remote_config,
                &["client", "config"],
                &instance.join("config"),
            )
            .await?;
            crate::drive::files::upload(
                &remote_config,
                &config.paths().manifest,
                &["manifest.toml"],
            )
            .await
        }
        super::ClientAction::Pull => {
            let mut remote_config = crate::drive::files::require_structure(config).await?;
            let neoforge_version =
                crate::minecraft::neoforge::ensure_version(&remote_config).await?;
            remote_config.neoforge_version = Some(neoforge_version);
            let java_path = crate::minecraft::java::ensure(&remote_config).await?;
            let instance = crate::prism::instance::provision(&remote_config, &java_path)?;
            crate::drive::files::sync_folder(
                &remote_config,
                &["client", "mods"],
                &instance.join("mods"),
            )
            .await?;
            crate::drive::files::sync_folder(
                &remote_config,
                &["client", "config"],
                &instance.join("config"),
            )
            .await?;
            crate::minecraft::manifest::validate(
                &remote_config,
                crate::minecraft::manifest::ModSide::Client,
                Some(&instance.join("mods")),
            )?;
            println!("Pulled client mods and config from Google Drive");
            Ok(())
        }
    }
}
