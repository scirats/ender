# Ender CLI

Ender CLI is a cross-platform command-line tool for managing a local Minecraft NeoForge installation with Prism Launcher and Google Drive storage.

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
ender-cli init
ender-cli status
ender-cli auth login
ender-cli server install
ender-cli server configure
ender-cli server start
ender-cli server stop
ender-cli server status
ender-cli server push
ender-cli server pull
ender-cli world push
ender-cli world pull
ender-cli client install
ender-cli client push
ender-cli client pull
```

Use `--root PATH` to select the local Ender CLI directory.

```text
ender-cli --root PATH init
```

## Initialization

`init` creates the local directory structure and a default manifest when they do not exist. Existing manifests are preserved. The command does not create or replace a world.

Existing installations keep their legacy local directory, Prism instance directory and Google Drive folder identifiers so worlds, mods and backups are not lost during the rename.

Place a Google OAuth desktop client secret at `client_secret.json` inside the Ender CLI root before running `ender-cli auth login`. The authenticated account must have access to the shared folder. A writable account can create backups; a read-only account can pull resources but cannot upload backups.

The first writable account creates the remote folder structure and uploads the manifest. `server push` and `client push` validate the local mod hashes and update the shared manifest with the files. Other accounts use the existing shared folder and download the manifest and resources.
