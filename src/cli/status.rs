use crate::config::AppConfig;
use crate::error::AppResult;

pub async fn run(config: &AppConfig) -> AppResult<()> {
    let paths = config.paths();
    println!("Root: {}", config.root.display());
    println!("Initialized: {}", paths.manifest.is_file());
    println!("Server directory: {}", paths.server.is_dir());
    println!("Client directory: {}", paths.client.is_dir());
    println!("World directory: {}", paths.world.is_dir());
    println!("Minecraft: {}", config.minecraft_version);
    println!("Java: {}", config.java_version);
    Ok(())
}
