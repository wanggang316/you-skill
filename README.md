# YouSkill

[![Version](https://img.shields.io/github/v/release/wanggang316/you-skill?label=Version)](https://github.com/wanggang316/you-skill/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS-lightgrey.svg)](https://github.com/wanggang316/you-skill/releases)
[![Built%20with%20Tauri](https://img.shields.io/badge/Built%20with-Tauri%202-orange.svg)](https://tauri.app/)

English | [中文](README_ZH.md)

YouSkill is a desktop [Agent Skills](https://agentskills.io/) manager that helps you organize all skills on your machine in one place. It is lightweight and fast, and aims to make skill management clear, visual, and maintainable.

## Why This Project

1. Most users work with multiple agent apps (Codex, Claude Code, Cursor, OpenCode, TRAE, etc.), but their skill systems are isolated and managed differently.
2. Installing, updating, and removing skills often means manual copying, syncing, and cleanup across many folders.
3. For non-technical users, manual installation or CLI-only workflows are still too complex.
4. High-quality skills are fragmented across sources, making discovery and reuse inefficient.

## Features

1. A central skill library at `~/.youskill/skills`: every skill is imported once and tracked by a single lock file (`~/.youskill/.skill-lock.json`).
2. Two-step workflow: import (marketplace, GitHub URL, `.zip` / `.skill` archives, local folders) and install (to user-level or project-level directories of 40+ developer tools, plus custom agent apps), by copy or symlink.
3. Install matrix per skill: see at a glance which projects and which agents a skill is installed to.
4. Change tracking: each install keeps the hub version it was written from, so the app can tell outdated, locally modified and conflicting copies apart and lets you choose the sync direction (update from source, push to targets, adopt a local copy).
5. Folder scanning: scan a project or an agent's skills directory and sync what it finds into the library.
6. Built-in dynamic skill marketplace (15,000+) with update checks against the source repository.
7. Compatible with the `.agents/skills` shared directory convention (the "Agents (shared)" target).
8. One-click backup of the whole library, multi-theme and multilingual UI.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the storage layout, lock file and drift model.

## Preview

![main](./public/01.png)
![install](./public/02.png)
![settings](./public/03.png)

## Download

Download installers from [Releases](https://github.com/wanggang316/you-skill/releases):

- **macOS**: `YouSkill_x.x.x_aarch64.dmg` (Apple Silicon) / `YouSkill_x.x.x_x64.dmg` (Intel)
- **Windows**: `YouSkill_x.x.x_x64-setup.exe`

## Tech Stack

- **Tauri v2**
- **Svelte 5**
- **Vite**
- **Tailwind CSS**
- **Lucide Icons**

## Project Structure

- `src/`: Frontend core
- `src-tauri/`: Rust backend and packaging config
- `scripts/`: Release and maintenance scripts

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) (recommended: 18+)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/)

### Install dependencies

```bash
npm install
```

### Run locally

```bash
npm run tauri -- dev
```

### Build

```bash
npm run build
npm run tauri -- build
```

### Release

1. Edit the `[Unreleased]` section in [CHANGELOG.md](CHANGELOG.md) with the changes for this release
2. Run the release script:
   ```bash
   npm run release <version>
   ```
   Example: `npm run release 0.9.0`

### Development Rules

[AGENTS.md](AGENTS.md)

## Roadmap

- [x] Project-Level skill management
- [x] Central skill library with change tracking and folder scanning
- [x] Skill details with support for viewing all skill files
- [ ] Backup to GitHub for Multi-Device sync
- [ ] AI-Powered find skill

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md)

## License

MIT
