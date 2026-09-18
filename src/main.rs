mod cli;
mod config;
mod drive;
mod error;
mod minecraft;
mod prism;
mod server;
mod system;
mod world;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::config::AppConfig;
use crate::error::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();
    let config = AppConfig::load(cli.root)?;

    match cli.command {
        Command::Init => cli::init::run(&config).await,
        Command::Status => cli::status::run(&config).await,
        Command::Auth(command) => cli::auth::run(&config, command.command).await,
        Command::Server(command) => cli::server::run(&config, command.command).await,
        Command::World(command) => cli::world::run(&config, command.command).await,
        Command::Client(command) => cli::client::run(&config, command.command).await,
    }
}
