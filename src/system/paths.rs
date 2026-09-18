use std::path::{Path, PathBuf};

pub fn server_mods(root: &Path) -> PathBuf {
    root.join("mods")
}

pub fn server_config(root: &Path) -> PathBuf {
    root.join("config")
}

pub fn client_mods(root: &Path) -> PathBuf {
    root.join("mods")
}

pub fn client_config(root: &Path) -> PathBuf {
    root.join("config")
}
