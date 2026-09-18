# Maincraft

Maincraft is a cross-platform CLI for managing a local Minecraft NeoForge installation with Prism Launcher and Google Drive storage.

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
maincraft init
maincraft status
maincraft auth login
maincraft server install
maincraft server configure
maincraft server start
maincraft server stop
maincraft server status
maincraft server push
maincraft server pull
maincraft world push
maincraft world pull
maincraft client install
maincraft client push
maincraft client pull
```

Use `--root PATH` to select the local Maincraft directory.

```text
maincraft --root PATH init
```

## Initialization

`init` creates the local directory structure and a default manifest when they do not exist. Existing manifests are preserved. The command does not create or replace a world.

Place a Google OAuth desktop client secret at `client_secret.json` inside the Maincraft root before running `maincraft auth login`. The authenticated account must have access to the shared `maincraft` folder. A writable account can create backups; a read-only account can pull resources but cannot upload backups.

The first writable account creates the remote folder structure and uploads the manifest. `server push` and `client push` validate the local mod hashes and update the shared manifest with the files. Other accounts use the existing shared folder and download the manifest and resources.
