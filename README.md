# Project Explainer OS

Local desktop app (Tauri + React + TypeScript) that imports a project folder, scans it with sensible ignore rules, sends a capped snapshot to the Claude API, stores structured JSON in SQLite, and shows a small personal library UI.

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+ and npm
- macOS / Windows / Linux (Tauri targets vary by platform)

## Claude API key

1. Copy `.env.example` to `.env` in the **project root** (next to `package.json`).
2. Set `ANTHROPIC_API_KEY` to your [Anthropic API key](https://console.anthropic.com/).
3. Optional: set `ANTHROPIC_MODEL` (default: `claude-3-5-sonnet-20241022`).

The Rust backend loads `.env` via `dotenvy` when you run `npm run tauri dev` or `npm run tauri build` from this directory. For a packaged app, set the variable in your shell or system environment instead.

## Install

```bash
cd project-explainer-os
npm install
```

## Run locally (development)

```bash
npm run tauri dev
```

This starts the Vite dev server and the Tauri shell. Use **Import Project** to pick a folder; the first analysis calls the Claude API.

## Build

```bash
CI=false npm run tauri build
```

Artifacts appear under `src-tauri/target/release/bundle/`. If your environment sets `CI=1`, some CLI versions parse it incorrectly; `CI=false` avoids that.

## Data storage

SQLite database path:

- **macOS:** `~/Library/Application Support/ProjectExplainerOS/project-explainer.db`
- **Windows:** `%LOCALAPPDATA%\ProjectExplainerOS\project-explainer.db`
- **Linux:** `~/.local/share/ProjectExplainerOS/project-explainer.db`

## Project layout

- `src/` — React UI (dashboard, project detail, routing).
- `src-tauri/src/` — Rust: `db` (SQLite), `scanner` (walk + ignore + file pick), `claude` (API + JSON validation), `lib` (Tauri commands).
- `src-tauri/tauri.conf.json` — Tauri configuration.
- `src-tauri/capabilities/` — Permissions (dialog).

## Icons

Placeholder icons live in `src-tauri/icons/`. Replace and run `npx @tauri-apps/cli icon path/to.png` to regenerate.
