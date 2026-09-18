use crate::config::AppConfig;
use crate::error::AppResult;

pub async fn run(config: &AppConfig, command: super::ServerAction) -> AppResult<()> {
    match command {
        super::ServerAction::Install => crate::minecraft::neoforge::install(config).await,
        super::ServerAction::Push => {
            if crate::server::lifecycle::running(config)? {
                return Err(crate::error::AppError::Process(
                    "stop the server before pushing server files".to_owned(),
                ));
            }
            let remote_config = crate::drive::files::require_structure(config).await?;
            crate::minecraft::manifest::validate(
                &remote_config,
                crate::minecraft::manifest::ModSide::Server,
                Some(&config.paths().server.join("mods")),
            )?;
            crate::drive::files::push_folder(
                &remote_config,
                &["server", "mods"],
                &config.paths().server.join("mods"),
            )
            .await?;
            crate::drive::files::push_folder(
                &remote_config,
                &["server", "config"],
                &config.paths().server.join("config"),
            )
            .await?;
            crate::drive::files::upload(
                &remote_config,
                &config.paths().manifest,
                &["manifest.toml"],
            )
            .await
        }
        super::ServerAction::Pull => {
            if crate::server::lifecycle::running(config)? {
                return Err(crate::error::AppError::Process(
                    "stop the server before pulling server files".to_owned(),
                ));
            }
            let remote_config = crate::drive::files::require_structure(config).await?;
            crate::drive::files::sync_folder(
                &remote_config,
                &["server", "mods"],
                &config.paths().server.join("mods"),
            )
            .await?;
            crate::drive::files::sync_folder(
                &remote_config,
                &["server", "config"],
                &config.paths().server.join("config"),
            )
            .await?;
            crate::minecraft::manifest::validate(
                &remote_config,
                crate::minecraft::manifest::ModSide::Server,
                Some(&config.paths().server.join("mods")),
            )?;
            println!("Pulled server mods and config from Google Drive");
            Ok(())
        }
        super::ServerAction::Start => crate::server::lifecycle::start(config).await,
        super::ServerAction::Stop => crate::server::lifecycle::stop(config),
        super::ServerAction::Status => crate::server::lifecycle::status(config).map(|_| ()),
        super::ServerAction::Configure { accept_eula } => {
            crate::server::configuration::interactive(config, accept_eula)
        }
    }
}
