# Architecture

YouSkill is a centralized skill manager: every managed skill lives once in a local hub,
and every agent app (user-level or per project) receives a copy or a symlink of that hub
entry. This document is the map of the storage layout, the lock file, and the drift model.
Code is the source of truth; when this document and the code disagree, fix the document.

## Storage layout

```
~/.youskill/
  skills/<name>/          # hub: the single canonical copy of each managed skill
  .skill-lock.json        # registry of sources, hashes and install targets
  instructions/<id>.md    # instruction templates: one Markdown file per template
  .instruction-lock.json  # registry of template names, hashes and install targets
  .trash/<name>-<stamp>/  # previous hub copy before a replacement (swept after 7 days)

<config_dir>/youskill/    # app config, custom agent apps, registered projects (unchanged)
```

Agent apps are defined in `src-tauri/src/services/agent_apps_service.rs`. Each app has a
user-level skills directory and a project-level one. Apps that natively read the shared
directories (`~/.agents/skills` and `.agents/skills`: Codex, Cursor, GitHub Copilot, VS
Code, Gemini CLI, OpenCode, Kimi, Warp, Zed) use them as their paths, so one install
serves every such app; `detect_path` (e.g. `~/.cursor`) decides whether the app is shown as
installed. Apps with their own directories (Claude Code, Cline, Windsurf, ...) keep them.
`LEGACY_USER_ROOTS` lists the app-specific directories those apps used before adopting the
shared one; migration and scanning still recognise skills there as targets of that app.
The built-in `agents` app is the shared directory itself as an explicit target. Each app
also carries the instruction files it reads: `profile_path` inside a project (`CLAUDE.md`
for Claude Code, `AGENTS.md` elsewhere) and `global_profile_path` at user level
(`~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md`). `list_memory_files` resolves them for one
scope and groups the agents that read the same file.

## Workspaces and projects

A workspace is a folder that holds projects, stored in `<config_dir>/youskill/user_workspaces.json`.
Scanning one walks up to four levels deep and reports a folder as a project when it has an
agent's project skills directory (`project_path`) or an agent instruction file
(`profile_path`); the walk stops at each project and skips hidden and excluded folders.
Registered projects (`user_projects.json`) keep the `workspacePath` they were found in, so
the projects page can group them; projects registered before workspaces existed have none.

## Lock file (`~/.youskill/.skill-lock.json`)

```json
{
  "version": 1,
  "migration": { "completedAt": "...", "imported": ["..."], "installs": 3, "errors": [] },
  "skills": {
    "<name>": {
      "source": { "type": "github", "repo": "owner/repo", "url": "...", "skillPath": "skills/foo/SKILL.md",
                  "branch": "main", "remoteSha": "...", "latestRemoteSha": "...", "checkedAt": "..." },
      "hash": "<sha256 of the hub directory at import / last accepted change>",
      "importedAt": "...",
      "updatedAt": "...",
      "installs": [
        { "scope": "user" | "project", "projectPath": "/abs" | null,
          "path": "/abs/target/dir", "mode": "copy" | "symlink",
          "hash": "<hub hash the target was written from>", "installedAt": "...",
          "agentIds": ["codex", "cursor"] }
      ]
    }
  }
}
```

- The record key is the skill name (SKILL.md frontmatter `name`, falling back to the
  directory name). Names are validated as safe directory names and compared
  case-insensitively.
- `source` is one of `github`, `folder`, `zip`, `none`. `folder` is only used for
  directories outside every agent root; a folder inside an agent root is registered as an
  install instead.
- Install records are keyed by target path. Several agent apps that share a directory
  (every agent using `.agents/skills` inside a project) share one record via `agentIds`.
  Uninstalling removes an agent id; files are deleted only when no agent is left.

## Content hash

`src-tauri/src/utils/hash.rs::hash_dir` is the only content hash: sha256 over entries
sorted by relative path (`/` separators), excluding `.git`, `node_modules`, `.DS_Store`,
`Thumbs.db`. Files contribute `rel 0x00 content 0xFF`, symlinks `rel 0x01 target 0xFF`.
A stat-only signature caches results in memory so listing stays cheap.

## Drift model

Three nodes per skill: source S, hub H, targets T. `list_hub_skills` computes everything
offline; `check_source_updates` is the only network call and stores its result in the lock.

| Node             | States                                                                                           | Rule                                                                                                                              |
| ---------------- | ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| Hub              | `ok`, `modified`, `missing`, `invalid`, `name_mismatch`                                          | `hash(H) != record.hash` is `modified`                                                                                            |
| Target (copy)    | `in_sync`, `outdated`, `modified`, `conflict`, `missing`                                         | three-way against `install.hash` as base: T==H in sync; T==base, H!=base outdated; T!=base, H==base modified; all differ conflict |
| Target (symlink) | `in_sync`, `broken_link`                                                                         | link into the hub is always in sync                                                                                               |
| Source           | `in_sync`, `update_available`, `source_modified`, `source_missing`, `not_checkable`, `unchecked` | github compares `remoteSha` with `latestRemoteSha`; folder hashes the folder                                                      |

Sync actions (`sync_skill`): `pull_source`, `push_targets`, `adopt_target`, `accept_hub`,
`push_source`. Any action that would discard local edits returns `applied: false` with
`blockers`; the caller may retry with `force`. Hub replacements move the old copy to
`.trash` first.

## Instructions: templates and agent files

Agent instruction files (`AGENTS.md`, `CLAUDE.md`, ...) are handled in two layers. The
**agent files** themselves, at user level (`global_profile_path`) and in every registered
project (`<project>/<profile_path>`), are listed by `list_instruction_files` and read and
written in place (`read_instruction_file`, `write_instruction_file`, which only accept
paths that are agent locations); nothing is copied anywhere for that. **Templates** are the
library: `~/.youskill/instructions/<id>.md`, keyed by a generated uuid, with a free-text
display name that need not be unique. `.instruction-lock.json` holds
`{ "instructions": { "<id>": { "name", "source", "hash", "importedAt", "updatedAt", "installs": [...] } } }`
with the same install records as skills (an `InstallRequest.name` carries the id). An
install target is the file an agent reads in a scope, so agents that read the same file
share one record. Because there is one file per agent location, installing where another
template is already installed is refused unless forced, which moves the install record
over; promoting an agent file to a template (`import_instructions` with the file's own
path) registers it as an install and takes the path over from any other template. Drift
is the same three-way comparison (`compare_three_way`) on file hashes with push, adopt and
accept, and `list_instruction_files` reports each file's template and state. The `source`
(`github` with repo, file path and branch; `file`; `agent` for a promoted agent file;
`none` when created in the app) is shown, nothing syncs with it. Imports come from picked
files or a folder (`detect_instruction_files`, Markdown files up to six levels deep,
skipping VCS, dependency and most hidden folders) or a GitHub repository, folder or file
URL (`detect_instruction_github`, downloaded to a temp directory). `write_instruction`
accepts new content as the template version and pushes it to copy targets that were in
sync; writing an agent file that is a symlink into the library does the same. Records
from before ids (keyed by a generated name, with every agent file copied in) are migrated
on the first listing: a record that only mirrored one agent file is dropped again, the
rest get an id and keep their key as name. Everything lives in `instruction_service`.

## Command surface

| Area         | Commands                                                                                                                                                                                                                                                                       |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Hub          | `list_hub_skills`, `get_hub_skill`, `import_skills`, `remove_hub_skill`, `sync_skill`, `diff_skill`, `check_source_updates`, `migrate_legacy`, `migration_status`                                                                                                              |
| Install      | `install_skill`, `uninstall_skill`                                                                                                                                                                                                                                             |
| Scan         | `scan_folder`, `import_scanned`                                                                                                                                                                                                                                                |
| Instructions | `list_instructions`, `get_instruction`, `read_instruction`, `write_instruction`, `rename_instruction`, `import_instructions`, `create_instruction`, `detect_instruction_files`, `detect_instruction_github`, `install_instruction`, `uninstall_instruction`, `remove_instruction`, `sync_instruction`, `diff_instruction`, `list_instruction_files`, `read_instruction_file`, `write_instruction_file` |
| Projects     | `list_user_projects`, `add_user_project`, `update_user_project`, `remove_user_project`, `list_workspaces`, `add_workspace`, `remove_workspace`, `scan_workspace`, `register_projects`, `list_memory_files`                                                                     |
| Detection    | `detect_github_manual`, `detect_github_auto`, `detect_zip`, `detect_folder` (stage into a temp dir)                                                                                                                                                                            |
| Other        | marketplace, agent apps, projects, settings, backup, file readers, translation                                                                                                                                                                                                 |

Services: `hub_service` (hub + lock records), `install_service` (targets), `drift_service`
(pure state computation), `diff_service` (file-level diff of the hub against a target or
source), `scan_service`, `migration_service`, `source_service` (network), `lock_service`
(atomic lock writes + process-wide operation mutex), `env.rs` (paths and agent/project
lists, injectable for tests).

## Migration from the previous layout

Before the hub, `~/.agents/skills` was the canonical directory and three lock files existed
(`~/.agents/.skill-lock.json`, `<config_dir>/youskill/native-skill-lock.json`,
`<project>/skills-lock.json`). On first list the app migrates lazily and idempotently:
hub copies are created, symlinks into the old canonical directory are relinked to the hub,
real copies become install records, and nothing legacy is deleted. The report is stored
under `migration` in the lock and shown once in the library.

## Frontend

- `src/lib/api/hub.ts` mirrors the command surface (camelCase payloads).
- `src/lib/stores/hub.ts` holds the skill list and agent apps; `stores/modals.ts` holds the
  import / install / force-confirm / diff modals and `performAction`, the helper that turns a
  blocked `ActionResult` into a confirmation.
- `src/lib/api/instructions.ts` and `stores/instructions.ts` do the same for the
  instruction library; the install, location, diff and remove dialogs take a `kind`
  (`skill` | `instruction`) and `confirmForce` is the shared force-confirmation helper.
- Routes: `/` library (skill list plus skill detail), `/instructions` (instruction list plus
  detail with a content preview), `/projects` install scopes (scope list
  plus a scope detail with its agents and skills; `?scope=user` or `?project=<path>` selects
  one), `/market`, `/skills/hub/<name>` and `/skills/remote/<name>` file viewers,
  `/settings`, `/agent-apps`.
