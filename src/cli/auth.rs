use crate::config::AppConfig;
use crate::error::AppResult;

pub async fn run(config: &AppConfig, command: super::AuthAction) -> AppResult<()> {
    match command {
        super::AuthAction::Login => crate::drive::auth::login(config).await,
        super::AuthAction::Logout => crate::drive::auth::logout(config).await,
    }
}
