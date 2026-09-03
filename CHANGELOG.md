# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes

- Managed skills now live in a central hub at `~/.youskill/skills/<name>` tracked by a single `~/.youskill/.skill-lock.json`; `~/.agents/skills` is an ordinary install target (built-in "Agents (shared)" app). The previous lock files (`~/.agents/.skill-lock.json`, `native-skill-lock.json`, `<project>/skills-lock.json`) are read once for migration and no longer written.
- Installing is split into two steps: import (GitHub / archive / folder into the hub) and install (hub into user-level or project agent directories). The `list_skills`, `install_from_*`, `manage_skill_agent_apps`, `check_skill_version`, `check_skills_updates` and `delete_skill` commands were replaced by the hub commands.

### Added

- Added bundled LobeHub icons with initial fallbacks to the Agent Apps page.
- Added a two-column Skill Library: skill list with search and filters on the left, and a detail pane on the right showing source, version hash, an install matrix (user level plus one row per project with agent icons), a changes panel and the SKILL.md preview.
- Added drift detection: every install target is compared three-way against the hub (in sync / outdated / modified / conflict / missing / broken link), GitHub sources can be checked for updates, and folder sources are compared by content hash. Sync actions (update from source, push to targets, adopt a target, accept hub edits, push to source) refuse to discard local changes unless confirmed.
- Added folder scanning: scan a project, an agent's skills directory or any folder, see which skills are new, identical, or different from the hub, and import, adopt, overwrite or register them as install targets.
- Added automatic, idempotent migration of the previous layout on first launch, with a one-time report in the library.
- Added a Marketplace page; importing from it goes through the hub and then offers to install.

### Changed

- Changed the default install mode to copy; symlink remains available per install.
- Changed backup to archive the whole hub (`~/.youskill`, skills and lock file).
- Changed GitHub imports to accept `/tree/<branch>/<path>` URLs and to record the branch that was downloaded.
- Changed the app to a wider two-column window with persistent navigation for adding skills, browsing the library, switching scopes, selecting projects, and viewing settings alongside the sidebar.
- Consolidated the two-column frame into the root Svelte layout so every route renders only its right-side page content and preserves shared sidebar state.
- Removed the brand logo from the sidebar while preserving the window drag area.
- Grouped the add action with the primary navigation below compact titlebar clearance and reduced single-line content header height.

### Fixed

- Fixed Agent Apps and Skill detail routes replacing the app shell instead of rendering in the right content pane.
- Fixed window dragging from sidebar and content headers by manually invoking Tauri window dragging from non-interactive header regions.
- Fixed archive extraction to reject entries that escape the destination directory.
- Fixed update checks for repositories whose default branch is not `main`.

### Removed

- Removed the Global / project scope switcher; the library shows every hub skill and can be filtered by project from the sidebar.
- Removed the "take over unmanaged skill" permission setting and the version-pick dialog; scanning replaces both.

## [0.8.5] - 2026-05-11

### Added
- Release v0.8.5



## [0.8.5] - 2026-05-11

### Added
- Added a one-click bulk update action for all local skills with available updates.

## [0.8.4] - 2026-03-01

### Added
- Added one-click translation for Markdown files in Skill directories.

## [0.8.3] - 2026-02-28

### Added
- Added a skill detail catalog drawer with full skill file tree navigation and in-page file switching.
- Added skill detail file preview modes for code and images, with unsupported-format fallback actions.

### Fixed
- Fixed project-scoped local skill detail pages showing `Skill not found` by preserving scope and project path in detail navigation.
- Fixed scope resetting to `Global` after returning from skill detail by restoring scope/project state from return URL.

## [0.8.2] - 2026-02-27

### Added
- Added project skill management.

## [0.8.1] - 2026-02-24

### Added
- Changed the GitHub download method from git clone to direct download.
- Added 20 agent apps.
- Fixed compatibility issue when the local skills directory is absent.

## [0.8.0] - 2026-02-23

### Added

- Initial formal release of YouSkill
- Core desktop workflow for skill management
