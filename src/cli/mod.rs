use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

pub mod auth;
pub mod client;
pub mod init;
pub mod server;
pub mod status;
pub mod world;

#[derive(Debug, Parser)]
#[command(
    name = "ender-cli",
    version,
    about = "Manage a local Minecraft NeoForge installation"
)]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub root: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Status,
    Auth(AuthCommand),
    Server(ServerCommand),
    World(WorldCommand),
    Client(ClientCommand),
}

#[derive(Debug, Args)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub command: AuthAction,
}

#[derive(Debug, Subcommand)]
pub enum AuthAction {
    Login,
    Logout,
}

#[derive(Debug, Args)]
pub struct ServerCommand {
    #[command(subcommand)]
    pub command: ServerAction,
}

#[derive(Debug, Subcommand)]
pub enum ServerAction {
    Install,
    Push,
    Pull,
    Start,
    Stop,
    Status,
    Configure {
        #[arg(long)]
        accept_eula: bool,
    },
}

#[derive(Debug, Args)]
pub struct WorldCommand {
    #[command(subcommand)]
    pub command: WorldAction,
}

#[derive(Debug, Subcommand)]
pub enum WorldAction {
    Push,
    Pull,
}

#[derive(Debug, Args)]
pub struct ClientCommand {
    #[command(subcommand)]
    pub command: ClientAction,
}

#[derive(Debug, Subcommand)]
pub enum ClientAction {
    Install,
    Push,
    Pull,
}
