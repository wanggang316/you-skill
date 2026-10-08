# Command Line Interface

`youskill` is a standalone binary that manages the same hub as the desktop app. It reads
and writes `~/.youskill` directly through the shared `youskill-core` crate; it does not
talk to a running app. Code is the source of truth; when this document and the code
disagree, fix the document.

## Goals

- One implementation of the hub, lock file and drift model, used by both the app and the
  CLI. The CLI must not fork any service logic.
- Safe to run while the app is open: every mutating operation holds a cross-process lock.
- Scriptable: stable `--json` output, meaningful exit codes, no prompts when stdin is not a
  terminal.

Out of scope for the first versions: the marketplace, and anything that needs a window
(translation, backup folder picker).

## Crate layout

`src-tauri/` is a Cargo workspace:

```
src-tauri/
  Cargo.toml                   # workspace + the Tauri app package (unchanged name)
  src/                         # Tauri app: main.rs, commands/*, tray.rs
  crates/youskill-core/        # services, models, utils, config, Env
  crates/youskill-cli/         # the youskill binary
```

- `youskill-core` has no dependency on `tauri`. The only async entry points
  (`source_service::pull_from_source`, `remote_service`) use `tokio`; both binaries own a
  runtime.
- The app re-exports the core modules at its crate root
  (`use youskill_core::{config, models, services, utils}`), so `commands/*` keep their
  `crate::services::...` paths. Commands stay thin adapters.
- `youskill-cli` depends on `clap` (derive), `serde_json`, `tokio` and `youskill-core`.
  Table output is hand-formatted; no TUI dependency.

## Cross-process lock

`lock_service::ops_guard` serialized mutating hub operations with a process-wide mutex.
With two processes that is not enough: a read-modify-write of `.skill-lock.json` in one
process can overwrite the other's result, and a hub directory can be replaced while the
other process copies it.

`ops_guard(env)` now takes the process mutex and then an exclusive advisory lock on
`~/.youskill/.ops.lock` (`std::fs::File::lock`). The guard owns the file handle; dropping
it releases the lock. The lock file is created on demand and never holds content.

Lock-file writes stay atomic (temp file + rename), so a reader that never takes the ops
lock (`list`, `show`, `status`) sees either the old or the new file, never a torn one.
A read may still observe a hub directory mid-replacement; that reports as `modified` or
`missing` for one listing and settles on the next.

The app refreshes its lists when the window regains focus so changes made by the CLI
appear without a restart.

## Environment

The CLI resolves paths exactly as the app does: `dirs_next::home_dir()` and
`dirs_next::config_dir()`. Integration tests isolate a run by setting `HOME` (and
`USERPROFILE` on Windows) to a temporary directory; no YouSkill-specific override exists.

Registered projects, agent apps, scan roots and settings are read from the same config
directory as the app (`<config_dir>/youskill`), so an install targeted with
`--project <path>` is recorded on the project the app shows.

## Command surface

Global flags: `--json` (machine output, camelCase, the same view structs the app uses),
`-y, --yes` (skip confirmations), `-q, --quiet`.

| Command | Service | Notes |
| --- | --- | --- |
| `youskill list [--state <s>,...]` | `hub_service::list_hub_skills` | Runs `ensure_migrated` and the trash sweep first, like the app. States filter on hub, target or source state. |
| `youskill show <name>` | `hub_service::get_hub_skill` | Source, hub state, install matrix with target states. |
| `youskill status [--exit-code]` | `list_hub_skills` | One line per skill that is not fully in sync. `--exit-code` returns 3 when any drift exists. |
| `youskill import <url\|zip\|dir>... [--pick <name>,...] [--overwrite]` | `skill_service::detect_*`, `source_service::detect_github_*`, `hub_service::import_skills` | A source that yields several skills imports all of them unless `--pick` narrows it; `--overwrite` replaces an existing hub copy (old copy goes to `.trash`). |
| `youskill install <name>... -a <agent>,... [-p [path]] [--mode copy\|symlink] [--force]` | `install_service::install_skill` | `-p` without a value means the current directory. `--mode` defaults to `sync_mode` from settings. `-a all` means every detected agent. |
| `youskill uninstall <name>... -a <agent>,... [-p [path]]` | `install_service::uninstall_skill` | Files are removed only when no agent is left on the record. |
| `youskill remove <name>... [--keep-targets]` | `hub_service::remove_hub_skill` | Removes the hub copy and every copy install; symlink targets are always removed because they would break. |
| `youskill update [--check] [name...]` | `source_service::check_source_updates`, `sync_skill(pull_source)` | No names means every skill with a checkable source. `--check` only refreshes the source state. |
| `youskill sync <name> <action> [--target <path>] [--force]` | `hub_service::sync_skill` | Actions: `pull-source`, `push-targets`, `adopt-target`, `accept-hub`, `push-source`. A blocked action prints the blockers and exits 2 unless `--force`. |
| `youskill diff <name> [--target <path> \| --source]` | `diff_service::diff_skill` | Unified diff on stdout; exits 0 when identical, 1 when different, like `diff(1)`. |
| `youskill scan <dir> [--depth <n>] [--import] [--register]` | `scan_service::scan_folder`, `import_scanned` | Prints what was found grouped by name. `--import` takes new skills into the hub; `--register` records known copies as installs. |
| `youskill agents [--all]` | `agent_apps_service::local_agent_apps` | Detected agents by default; `--all` includes undetected ones. |
| `youskill projects list\|add <path>\|remove <path>` | `user_projects_service` | |
| `youskill completions <shell>` | `clap_complete` | |

Confirmations: `remove`, `uninstall` with files to delete, and any `--force` action ask
once when stdin is a terminal. Without a terminal and without `-y` they exit 2 and print
what they would have done.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. For `diff`: no difference. |
| 1 | Error (invalid input, I/O, network). For `diff`: files differ. |
| 2 | Blocked: an action returned `applied: false` with blockers, or a confirmation was needed and stdin is not a terminal. |
| 3 | Drift detected (`status --exit-code` only). |

Errors go to stderr as one line; with `--json` the same message is wrapped as
`{"error": "..."}`.

## Output

Human output is a fixed-width table per command, states in the words the app uses
(`in_sync`, `outdated`, `modified`, `conflict`, `missing`, `update_available`, ...). The
columns and widths are not a contract; `--json` is. The JSON shape is the serde view of
the service result (`HubSkillView`, `ActionResult`, `ImportOutcome`, `ScanItem`, ...), so
the app and the CLI cannot disagree about a state.

## Distribution

1. The release workflow builds `youskill` for macOS (arm64, x64) and Windows (x64) and
   attaches the archives to the same GitHub release as the installers. A Homebrew tap
   follows once the binary is stable.
2. Later: the app bundles the binary as a Tauri `externalBin` and offers
   "Install command line tool" in settings, which links it into `/usr/local/bin`.

## Milestones

| Milestone | Content | Verification |
| --- | --- | --- |
| M0 | Workspace split into `youskill-core` and the app; no behavior change. `tauri::async_runtime` replaced by `tokio` in core. | `npm run check`, `npm run test`, app smoke test. |
| M1 | Cross-process lock. CLI binary with read-only commands: `list`, `show`, `status`, `diff`, `agents`, `projects list`. | Two processes installing concurrently lose no record. Integration tests with a temp `HOME`. |
| M2 | Mutating commands: `import`, `install`, `uninstall`, `remove`, `update`, `sync`, `scan`, `projects add/remove`. App refreshes on focus. | Integration tests per command, including blocked and forced paths. |
| M3 | `completions`, release artifacts, Homebrew tap, "Install command line tool" in the app. Instruction commands (`instructions list/install/...`). | Release pipeline run. |
