use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

pub fn configure(config: &AppConfig, accept_eula: bool) -> AppResult<()> {
    apply(config, accept_eula)
}

pub fn interactive(config: &AppConfig, accept_eula: bool) -> AppResult<()> {
    let mut updated = config.clone();
    println!("Configure the Minecraft server. Press Enter to keep the current value.");
    updated.server_port = prompt_number("Server port", config.server_port)?;
    updated.world_name = prompt_text("World name", &config.world_name)?;
    updated.minimum_memory_mb = prompt_number("Minimum memory (MB)", config.minimum_memory_mb)?;
    updated.maximum_memory_mb = prompt_number("Maximum memory (MB)", config.maximum_memory_mb)?;
    updated.eula_accepted = if accept_eula {
        true
    } else {
        prompt_bool("Accept Minecraft EULA", config.eula_accepted)?
    };
    updated.validate()?;
    updated.save()?;
    apply(&updated, false)?;
    println!(
        "Server configuration saved to {}",
        updated.paths().server.display()
    );
    Ok(())
}

fn apply(config: &AppConfig, accept_eula: bool) -> AppResult<()> {
    let paths = config.paths();
    let mut properties = read_properties(&paths.server_properties)?;
    properties.insert("server-port".to_owned(), config.server_port.to_string());
    properties.insert("level-name".to_owned(), config.world_name.clone());
    write_properties(&paths.server_properties, &properties)?;

    let jvm_args = format!(
        "-Xms{}M\n-Xmx{}M\n",
        config.minimum_memory_mb, config.maximum_memory_mb
    );
    fs::write(&paths.server_jvm_args, jvm_args)
        .map_err(|error| AppError::io(&paths.server_jvm_args, error))?;

    if accept_eula {
        let mut updated = config.clone();
        updated.eula_accepted = true;
        updated.save()?;
        fs::write(&paths.eula, "eula=true\n").map_err(|error| AppError::io(&paths.eula, error))?;
    } else if config.eula_accepted {
        fs::write(&paths.eula, "eula=true\n").map_err(|error| AppError::io(&paths.eula, error))?;
    }
    Ok(())
}

fn prompt_text(label: &str, current: &str) -> AppResult<String> {
    let value = prompt(label, current)?;
    if value.is_empty() {
        return Ok(current.to_owned());
    }
    Ok(value)
}

fn prompt_number<T>(label: &str, current: T) -> AppResult<T>
where
    T: std::fmt::Display + std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = prompt(label, current.to_string())?;
    if value.is_empty() {
        return Ok(current);
    }
    value
        .parse()
        .map_err(|error| AppError::Configuration(format!("invalid {label}: {error}")))
}

fn prompt_bool(label: &str, current: bool) -> AppResult<bool> {
    let default = if current { "Y/n" } else { "y/N" };
    print!("{label}? [{default}]: ");
    io::stdout()
        .flush()
        .map_err(|error| AppError::io("stdout", error))?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| AppError::io("stdin", error))?;
    match answer.trim().to_ascii_lowercase().as_str() {
        "" => Ok(current),
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => Err(AppError::Configuration(format!(
            "invalid answer for {label}"
        ))),
    }
}

fn prompt<T: std::fmt::Display>(label: &str, current: T) -> AppResult<String> {
    print!("{label} [{current}]: ");
    io::stdout()
        .flush()
        .map_err(|error| AppError::io("stdout", error))?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .map_err(|error| AppError::io("stdin", error))?;
    Ok(value.trim().to_owned())
}

fn read_properties(path: &std::path::Path) -> AppResult<BTreeMap<String, String>> {
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let content = fs::read_to_string(path).map_err(|error| AppError::io(path, error))?;
    Ok(content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect())
}

fn write_properties(
    path: &std::path::Path,
    properties: &BTreeMap<String, String>,
) -> AppResult<()> {
    let content = properties
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{content}\n")).map_err(|error| AppError::io(path, error))
}
