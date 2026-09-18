use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const APP_DIRECTORY: &str = "ender";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub root: PathBuf,
    pub minecraft_version: String,
    pub neoforge_version: Option<String>,
    pub java_version: u8,
    pub world_name: String,
    pub server_port: u16,
    pub minimum_memory_mb: u32,
    pub maximum_memory_mb: u32,
    pub eula_accepted: bool,
    pub drive_folder_id: Option<String>,
    #[serde(default)]
    pub drive_server_folder_id: Option<String>,
    #[serde(default)]
    pub drive_client_folder_id: Option<String>,
    #[serde(default)]
    pub drive_backups_folder_id: Option<String>,
    #[serde(default)]
    pub drive_server_mods_folder_id: Option<String>,
    #[serde(default)]
    pub drive_server_config_folder_id: Option<String>,
    #[serde(default)]
    pub drive_client_mods_folder_id: Option<String>,
    #[serde(default)]
    pub drive_client_config_folder_id: Option<String>,
    pub prism_root: Option<PathBuf>,
}

impl AppConfig {
    pub fn load(root: Option<PathBuf>) -> AppResult<Self> {
        let root = root.unwrap_or_else(default_root);
        let config_path = root.join(CONFIG_FILE);

        if config_path.is_file() {
            let content = fs::read_to_string(&config_path)
                .map_err(|error| AppError::io(&config_path, error))?;
            let mut config: Self = toml::from_str(&content)?;
            config.root = root;
            config.validate()?;
            return Ok(config);
        }

        Ok(Self::new(root))
    }

    pub fn save(&self) -> AppResult<()> {
        self.validate()?;
        fs::create_dir_all(&self.root).map_err(|error| AppError::io(&self.root, error))?;
        let path = self.root.join(CONFIG_FILE);
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content).map_err(|error| AppError::io(path, error))
    }

    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            minecraft_version: "1.21.1".to_owned(),
            neoforge_version: None,
            java_version: 21,
            world_name: "world".to_owned(),
            server_port: 25565,
            minimum_memory_mb: 2048,
            maximum_memory_mb: 4096,
            eula_accepted: false,
            drive_folder_id: None,
            drive_server_folder_id: None,
            drive_client_folder_id: None,
            drive_backups_folder_id: None,
            drive_server_mods_folder_id: None,
            drive_server_config_folder_id: None,
            drive_client_mods_folder_id: None,
            drive_client_config_folder_id: None,
            prism_root: None,
        }
    }

    pub fn paths(&self) -> AppPaths {
        AppPaths::new(&self.root)
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.minimum_memory_mb == 0 || self.minimum_memory_mb > self.maximum_memory_mb {
            return Err(AppError::Configuration(
                "minimum memory must be greater than zero and no greater than maximum memory"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub server: PathBuf,
    pub client: PathBuf,
    pub world: PathBuf,
    pub backups: PathBuf,
    pub java: PathBuf,
    pub manifest: PathBuf,
    pub server_pid: PathBuf,
    pub server_log: PathBuf,
    pub server_properties: PathBuf,
    pub server_jvm_args: PathBuf,
    pub eula: PathBuf,
    pub google_client_secret: PathBuf,
    pub google_token: PathBuf,
}

impl AppPaths {
    fn new(root: &Path) -> Self {
        let default_secret = root.join("client_secret.json");
        let google_client_secret = if default_secret.is_file() {
            default_secret
        } else if let Ok(current_directory) = std::env::current_dir() {
            current_directory.join("client_secret.json")
        } else {
            root.join("client_secret.json")
        };
        Self {
            root: root.to_owned(),
            server: root.join("server"),
            client: root.join("client"),
            world: root.join("world"),
            backups: root.join("backups"),
            java: root.join("java"),
            manifest: root.join("manifest.toml"),
            server_pid: root.join("server").join("server.pid"),
            server_log: root.join("server").join("server.log"),
            server_properties: root.join("server").join("server.properties"),
            server_jvm_args: root.join("server").join("user_jvm_args.txt"),
            eula: root.join("server").join("eula.txt"),
            google_client_secret,
            google_token: root.join("google-token.json"),
        }
    }
}

fn default_root() -> PathBuf {
    ProjectDirs::from("", "", APP_DIRECTORY)
        .map(|directories| directories.data_local_dir().to_owned())
        .unwrap_or_else(|| PathBuf::from(APP_DIRECTORY))
}
