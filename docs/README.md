# AutoLaunch Architecture

## Overview

AutoLaunch is a Tauri desktop application that clones a GitHub repository, analyzes its stack, prepares an execution environment, starts the project, and exposes runtime status back to the UI.

Primary layers:

1. React frontend in `src/`
2. Tauri IPC bridge between frontend and Rust commands
3. Rust application core in `src-tauri/src/`
4. SQLite persistence for projects, snapshots, trusted repositories, and metadata

## Runtime Flow

1. The user submits a GitHub URL in `src/App.tsx`.
2. Frontend calls `analyze_repository` via Tauri `invoke`.
3. Rust normalizes the GitHub URL, clones the repository, analyzes project files, and stores project metadata in SQLite.
4. Backend returns `project_id` and `project_info` to the UI.
5. Frontend starts the project with `start_project`, optionally passing a manual command override.
6. Backend creates an environment, launches the process, captures logs, detects ports, and tracks status in memory.
7. Frontend polls `get_project_status` and `get_process_logs` to render the live process window.

## Frontend Structure

- `src/App.tsx` - main orchestration for analysis, launch, restart, stop, history modal, settings modal, and process window
- `src/components/ProcessWindow.tsx` - runtime progress and log viewer
- `src/components/ProjectManager.tsx` - project history, search, tags, and relaunch actions
- `src/components/SecurityWarnings.tsx` - trusted repository and warning presentation
- `src/components/Settings.tsx` - app settings UI
- `src/styles.css` and component CSS files - shared and component-specific styling

## Backend Modules

- `src-tauri/src/commands.rs` - Tauri command surface used by the frontend
- `src-tauri/src/url_parser.rs` - GitHub URL parsing and normalization
- `src-tauri/src/project_analyzer.rs` - stack detection, dependency discovery, config file discovery, and default entry command detection
- `src-tauri/src/environment_manager.rs` - direct and sandbox environment preparation
- `src-tauri/src/process_controller.rs` - process lifecycle, log capture, port detection, restart, and browser opening
- `src-tauri/src/security_scanner.rs` - static command and project risk checks plus trusted repository support
- `src-tauri/src/snapshot_manager.rs` - project snapshot creation, loading, deletion, and cleanup
- `src-tauri/src/settings_manager.rs` - persisted application settings
- `src-tauri/src/database.rs` - SQLite access layer
- `src-tauri/src/models.rs` - shared data contracts exchanged between layers
- `src-tauri/src/error.rs` - domain error types and user-friendly error context

## Persistence

SQLite stores:

- analyzed projects and launch history
- trusted repositories
- snapshot metadata
- project tags and search metadata

Runtime process handles and in-memory environment state are kept in global managers inside `src-tauri/src/commands.rs`.

## Key User-Facing Capabilities

- GitHub URL parsing in both full URL and `owner/repo` form
- stack detection for Node.js, Python, Rust, Go, Java, Docker, and static sites
- environment setup and process start with optional manual command override
- live process logs and status polling in the UI
- port detection and automatic browser opening for local apps
- project history with search and tags
- snapshot save/load flow
- trusted repository management and security warnings
- persistent application settings

## Source Of Truth Notes

- This file is the canonical high-level architecture document for the project.
- Delivery readiness and progress tracking are maintained in `memory_bank/projectbrief.md`.
- Product intent and execution context are tracked in `memory_bank/productContext.md` and `memory_bank/activeContext.md`.
- Product requirements are described in `PRD.md`.

## Current Limitations

- Windows Tauri build and some Rust verification steps require Visual Studio with the C++ toolchain.
- The frontend currently relies on polling for runtime updates; there is no event-driven streaming channel yet.
- Docker-based isolation depends on a preinstalled local Docker runtime.
