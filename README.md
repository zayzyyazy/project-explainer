# Project Explainer OS

Local desktop app (Tauri + Rust + React + TypeScript) that turns finished repos into **structured project intelligence** — case studies, opportunities, and portfolio-ready copy. BYOK: OpenAI or Anthropic.

**Use case:** You ship well but explaining past work for proposals, interviews, or your portfolio is slow. Import a folder, get grounded Problem → Solution → Outcome narratives stored in SQLite on your machine.

## What it does

### Import & analyze
- Pick a project folder → Rust scanner (ignore rules, stack detection, capped file snapshot)
- AI analysis → structured JSON: summary, problem, value, capabilities, architecture, product intelligence, interview lines
- Personal project library with pin, rename, re-analyze

### Case studies
- Client-ready copy: problem, stakes, approach, outcome, narrative
- Illustrative proof blocks (CLI / file / UI examples inferred from the repo)
- Optional **writer profile** tunes tone (freelancer / indie / developer)
- Cached results with regenerate

### Opportunities
- AI-generated business ideas *from* a project (pricing, target customer, validation steps)
- Save opportunities as **Idea Projects** for later

### Living system
- **Rank top projects** for your stated goal
- **Incremental update** — re-scan and record what changed
- **Evolution suggestions** and **positioning clarity**

### Export bundle
- Export markdown/text: `case-study.md`, `short-pitch.txt`, optional `opportunities.md`

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+ and npm
- macOS / Windows / Linux

## API keys

1. Copy `.env.example` to `.env` in the project root (or `src-tauri/.env`).
2. Set `ANTHROPIC_API_KEY` and/or `OPENAI_API_KEY`.
3. Optional: `AI_PROVIDER` (`anthropic` | `openai`), `ANTHROPIC_MODEL`, `OPENAI_MODEL`.
4. Or configure provider + key in **Settings** inside the app (stored locally in SQLite).

## Install & run

```bash
npm install
npm run tauri:dev
```

## Build

```bash
CI=false npm run tauri build
```

Artifacts: `src-tauri/target/release/bundle/`

## Data storage

SQLite database:

- **macOS:** `~/Library/Application Support/ProjectExplainerOS/project-explainer.db`
- **Windows:** `%LOCALAPPDATA%\ProjectExplainerOS\project-explainer.db`
- **Linux:** `~/.local/share/ProjectExplainerOS/project-explainer.db`

## Project layout

- `src/` — React UI (landing, dashboard, project detail, case study, opportunities, settings)
- `src-tauri/src/` — Rust: `db`, `scanner`, `claude`, `openai`, `case_study`, `opportunities`, `living`
- `src-tauri/capabilities/` — Tauri permissions

## Tech stack

Tauri 2 · Rust · React · TypeScript · Vite · SQLite · OpenAI / Anthropic APIs
