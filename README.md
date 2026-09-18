# Ender

Ender is a cross-platform command-line tool for managing a local Minecraft NeoForge installation with Prism Launcher and Google Drive storage.

## Current Scope

- Minecraft Java Edition 1.21.1
- NeoForge 21.1.x
- Java 21
- macOS, Linux, and Windows targets
- Idempotent local initialization
- Separate server and client directories
- Separate server and client mod directories
- Google Drive OAuth with a shared folder as the remote source
- Compressed world backups with server-state protection
- Prism Launcher instance provisioning

## Usage

```text
ender init
ender status
ender auth login
ender server install
ender server configure
ender server start
ender server stop
ender server status
ender server push
ender server pull
ender world push
ender world pull
ender client install
ender client push
ender client pull
```

Use `--root PATH` to select the local Ender directory.

```text
ender --root PATH init
```

## Initialization

`init` creates the local directory structure and a default manifest when they do not exist. Existing manifests are preserved. The command does not create or replace a world.

Place a Google OAuth desktop client secret at `client_secret.json` inside the Ender root before running `ender auth login`. The authenticated account must have access to the shared Ender folder. A writable account can create backups; a read-only account can pull resources but cannot upload backups.

The first writable account creates the remote Ender folder structure and uploads the manifest. `server push` and `client push` validate the local mod hashes and update the shared manifest with the files. Other accounts use the existing shared folder and download the manifest and resources.

## Releases and packages

Push a tag such as `v0.1.0` to start `.github/workflows/release.yml`. It builds archives for Linux, macOS, and Windows and publishes SHA256 checksums plus a ready-to-submit Scoop manifest. The package definitions in `packaging/` are templates for a Homebrew tap, AUR, and Scoop bucket; update their version and checksums for each release.

Release builds target Linux, Windows, and Apple Silicon macOS.

### Linux x86_64

```bash
mkdir -p "$HOME/.local/bin"
curl -fsSL https://github.com/scirats/ender/releases/download/v0.1.0/ender-0.1.0-x86_64-unknown-linux-gnu.tar.gz \
  | tar -xz -C "$HOME/.local/bin"
chmod +x "$HOME/.local/bin/ender"
```

Ensure `~/.local/bin` is in your `PATH`, then run:

```bash
ender --version
```

### Arch Linux

```bash
sudo pacman -U https://github.com/scirats/ender/releases/download/v0.1.0/ender-0.1.0-1-x86_64.pkg.tar.zst
```

### macOS Apple Silicon

```bash
mkdir -p "$HOME/.local/bin"
curl -fsSL https://github.com/scirats/ender/releases/download/v0.1.0/ender-0.1.0-aarch64-apple-darwin.tar.gz \
  | tar -xz -C "$HOME/.local/bin"
chmod +x "$HOME/.local/bin/ender"
```

Ensure `~/.local/bin` is in your `PATH`, then run:

```bash
ender --version
```

### Windows x86_64

Run in PowerShell:

```powershell
$installDir = "$HOME\bin\ender"
$archive = "$env:TEMP\ender-0.1.0.zip"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Invoke-WebRequest -Uri "https://github.com/scirats/ender/releases/download/v0.1.0/ender-0.1.0-x86_64-pc-windows-msvc.zip" -OutFile $archive
Expand-Archive -Path $archive -DestinationPath $installDir -Force
Remove-Item $archive
```

Add `$HOME\bin\ender` to your user `PATH`, open a new PowerShell window, and run:

```powershell
ender.exe --version
```
