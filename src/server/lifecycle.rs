use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::process::Stdio;

use sysinfo::{Pid, ProcessesToUpdate, Signal, System};
use tokio::process::Command;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::minecraft::java;
use crate::server::configuration;

pub async fn start(config: &AppConfig) -> AppResult<()> {
    if running(config)? {
        return Err(AppError::Process(
            "the server is already running".to_owned(),
        ));
    }
    let mut config = config.clone();
    if !config.eula_accepted {
        print!("Accept the Minecraft EULA to start the server? [y/N]: ");
        io::stdout()
            .flush()
            .map_err(|error| AppError::io("stdout", error))?;
        let mut answer = String::new();
        io::stdin()
            .read_line(&mut answer)
            .map_err(|error| AppError::io("stdin", error))?;
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            return Err(AppError::Configuration(
                "Minecraft EULA was not accepted".to_owned(),
            ));
        }
        config.eula_accepted = true;
        config.save()?;
    }
    configuration::configure(&config, false)?;
    let paths = config.paths();
    let java_path = java::ensure(&config).await?;
    let path = prepend_java_to_path(&java_path)?;
    let neoforge_version = config
        .neoforge_version
        .as_deref()
        .ok_or_else(|| AppError::Configuration("NeoForge server is not installed".to_owned()))?;
    let arguments_file = if cfg!(windows) {
        format!("libraries/net/neoforged/neoforge/{neoforge_version}/win_args.txt")
    } else {
        format!("libraries/net/neoforged/neoforge/{neoforge_version}/unix_args.txt")
    };
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.server_log)
        .map_err(|error| AppError::io(&paths.server_log, error))?;
    let child = Command::new(&java_path)
        .args(["@user_jvm_args.txt", &format!("@{arguments_file}")])
        .current_dir(&paths.server)
        .env("PATH", path)
        .kill_on_drop(false)
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            log.try_clone()
                .map_err(|error| AppError::io(&paths.server_log, error))?,
        ))
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|error| AppError::Process(error.to_string()))?;
    let pid = child
        .id()
        .ok_or_else(|| AppError::Process("could not determine the server process ID".to_owned()))?;
    fs::write(&paths.server_pid, pid.to_string())
        .map_err(|error| AppError::io(&paths.server_pid, error))?;
    let java_identity = server_java_path(&config);
    fs::write(&java_identity, java_path.to_string_lossy().as_bytes())
        .map_err(|error| AppError::io(&java_identity, error))?;
    for _ in 0..100 {
        if process_exists(pid) {
            println!("Minecraft server started with PID {}", pid);
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let _ = fs::remove_file(&paths.server_pid);
    let _ = fs::remove_file(server_java_path(&config));
    Err(AppError::Process(
        "the server exited during startup; check server.log".to_owned(),
    ))
}

pub fn stop(config: &AppConfig) -> AppResult<()> {
    let pid = read_pid(config)?;
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    let process = system
        .process(Pid::from_u32(pid))
        .ok_or_else(|| AppError::Process("the server process is not running".to_owned()))?;
    if !is_server_process(config, process) {
        return Err(AppError::Process(
            "the PID file does not identify the Minecraft server".to_owned(),
        ));
    }
    if !process
        .kill_with(Signal::Term)
        .unwrap_or_else(|| process.kill())
    {
        return Err(AppError::Process(
            "the server process could not be stopped".to_owned(),
        ));
    }
    for _ in 0..50 {
        system.refresh_processes(ProcessesToUpdate::All, true);
        if system.process(Pid::from_u32(pid)).is_none() {
            fs::remove_file(&config.paths().server_pid)
                .map_err(|error| AppError::io(config.paths().server_pid, error))?;
            fs::remove_file(server_java_path(config))
                .map_err(|error| AppError::io(server_java_path(config), error))?;
            println!("Minecraft server stopped");
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(AppError::Process(
        "the Minecraft server did not exit after stop was requested".to_owned(),
    ))
}

pub fn status(config: &AppConfig) -> AppResult<bool> {
    let running = running(config)?;
    println!(
        "Minecraft server: {}",
        if running { "running" } else { "stopped" }
    );
    Ok(running)
}

pub fn running(config: &AppConfig) -> AppResult<bool> {
    let pid = match read_pid(config) {
        Ok(pid) => pid,
        Err(_) => return Ok(false),
    };
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    Ok(system
        .process(Pid::from_u32(pid))
        .is_some_and(|process| is_server_process(config, process)))
}

fn is_server_process(config: &AppConfig, process: &sysinfo::Process) -> bool {
    let identity = fs::read_to_string(server_java_path(config)).ok();
    let executable = process.exe().and_then(|path| path.to_str());
    identity
        .as_deref()
        .zip(executable)
        .is_some_and(|(expected, actual)| expected == actual)
}

fn process_exists(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system.process(Pid::from_u32(pid)).is_some()
}

fn server_java_path(config: &AppConfig) -> std::path::PathBuf {
    config.paths().server.join("server.java")
}

fn read_pid(config: &AppConfig) -> AppResult<u32> {
    let path = config.paths().server_pid;
    let value = fs::read_to_string(&path).map_err(|error| AppError::io(&path, error))?;
    value
        .trim()
        .parse()
        .map_err(|error| AppError::Configuration(format!("invalid server PID: {error}")))
}

fn prepend_java_to_path(java: &std::path::Path) -> AppResult<std::ffi::OsString> {
    let Some(java_directory) = java.parent() else {
        return env::var_os("PATH")
            .ok_or_else(|| AppError::Configuration("PATH is not configured".to_owned()));
    };
    let mut paths = vec![java_directory.to_owned()];
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    env::join_paths(paths).map_err(|error| AppError::Configuration(error.to_string()))
}
