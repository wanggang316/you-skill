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

1. Install skills for 40+ popular developer tools, with support for custom agent apps.
2. Compatible with Vercel `skill add` conventions and directory structure (`.agents/skills` as a shared standard).
3. Automatic update checks with one-click skill upgrades.
4. Built-in dynamic skill marketplace (15,000+) for fast search and install.
5. Install from marketplace, GitHub URL, local archives (`.zip` / `.skill`), or local folders.
6. One-click skill backup.
7. Two sync modes: Symlink and Copy, so one change can be reflected across multiple apps.
8. Multi-theme and multilingual UI.

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

### Development Rules

[AGENTS.md](AGENTS.md)

## Roadmap

- [x] Project-Level skill management
- [ ] Backup to GitHub for Multi-Device sync
- [ ] Skill details with support for viewing all skill files
- [ ] AI-Powered find skill

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md)

## License

MIT
