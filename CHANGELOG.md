# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added bundled LobeHub icons with initial fallbacks to the Agent Apps page.

### Changed

- Changed the app to a wider two-column window with persistent navigation for adding skills, browsing the library, switching scopes, selecting projects, and viewing settings alongside the sidebar.
- Removed the brand logo from the sidebar while preserving the window drag area.
- Grouped the add action with the primary navigation below compact titlebar clearance and reduced single-line content header height.

### Fixed

- Fixed Agent Apps and Skill detail routes replacing the app shell instead of rendering in the right content pane.
- Fixed window dragging from sidebar and content headers by manually invoking Tauri window dragging from non-interactive header regions.

### Removed


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
