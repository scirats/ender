use crate::config::AppConfig;
use crate::error::AppResult;

pub async fn run(config: &AppConfig, command: super::WorldAction) -> AppResult<()> {
    match command {
        super::WorldAction::Push => crate::world::push::run(config).await,
        super::WorldAction::Pull => crate::world::pull::run(config).await,
    }
}
