# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Added a skill detail catalog drawer with full skill file tree navigation and in-page file switching.
- Added skill detail file preview modes for code and images, with unsupported-format fallback actions.

### Changed

### Fixed
- Fixed project-scoped local skill detail pages showing `Skill not found` by preserving scope and project path in detail navigation.
- Fixed scope resetting to `Global` after returning from skill detail by restoring scope/project state from return URL.

### Removed


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
