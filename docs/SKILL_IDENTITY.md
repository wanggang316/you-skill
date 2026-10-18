# Skill Identity

Status: proposed. This document describes how the hub separates the identity of a skill
from its name. When the change is implemented, move the stable parts into
[ARCHITECTURE.md](ARCHITECTURE.md) and [CLI.md](CLI.md) and keep this file as the design
record.

## Problem

The skill name (SKILL.md frontmatter `name`, else the directory name) has four roles:

| Role                     | Where                                            | Required uniqueness  |
| ------------------------ | ------------------------------------------------ | -------------------- |
| Lock record key          | `.skill-lock.json` `skills.<name>`               | whole hub (identity) |
| Hub directory            | `~/.youskill/skills/<name>/`                     | whole hub (identity) |
| Install target directory | `install_service::resolve_target`: `root/<name>` | one agent root       |
| Agent runtime name       | frontmatter `name` (for example `/tdd`)          | one agent scope      |

Because one value has all four roles, the hub accepts only one skill per name. Two
different skills with the same name from two sources (for example `tdd` from
`backnotprop/pstack` and `tdd` from another repository) cannot both be in the hub. The
import must replace the existing skill or fail (`hub_service::import_one`).

Only the last two roles need a unique name, and only inside one agent root. The hub must
keep both skills, and the user must be able to install each one in a different scope.

## Goals

- The hub can hold more than one skill with the same name.
- Each skill has a stable id. The id is the key of the lock record and the hub directory.
- The name stays the install directory name and the agent runtime name. The content of
  SKILL.md is not changed.
- Two skills with the same name cannot be installed into the same agent root at the same
  time. This check moves from import to install.
- Existing hubs work without a move of directories or a relink of symlinks.
- Existing CLI scripts that use a skill name continue to work while the name is unique.

## Non-goals

- Rename on install (an install directory name that is different from the skill name).
  Agents read the frontmatter `name`, so a different directory name does not remove the
  conflict at runtime.
- Detection of shadowing across scopes (a user-level `tdd` and a project-level `tdd` that
  the same agent reads). The agent decides which one wins.
- Change of the instruction library. It already uses ids.

## Model

```
SkillRecord (key: id)
  name     install directory name and agent runtime name
  source   github | folder | zip | none
  hash, importedAt, updatedAt
  installs[]   each install path is <agent root>/<name>
```

- `id` is the lock key and the hub directory name: `~/.youskill/skills/<id>/`.
- `name` is stored in the record. `hub_state` is `name_mismatch` when the frontmatter
  name of the hub copy is different from `record.name` (today it compares with the key).
- The id does not change after it is created. A pull from the source does not change it.

### Id format

The id is a readable slug:

1. Use `name` when no record key and no hub directory has the same value. The comparison
   ignores case, because the default macOS and Windows file systems ignore case.
2. Else use `<name>-2`, `<name>-3`, ... and select the first free value.

The id goes through the same validation as a name (`validate_skill_name`).

Why not a UUID (the instruction library uses UUIDs):

- Legacy records keep `id = name`. Thus the migration does not move hub directories and
  does not relink symlink installs.
- Symlink targets (`~/.claude/skills/tdd -> ~/.youskill/skills/tdd-2`) and CLI arguments
  stay readable.

The cost: an id can look like a name of a different skill (a skill can have the name
`tdd-2`). The CLI rule "an exact id match wins" (below) makes this deterministic, and
`list` shows the id next to the name when they are different.

## Lock file

```json
{
  "version": 1,
  "skills": {
    "tdd":   { "name": "tdd", "source": { "type": "github", "repo": "mattpocock/skills", ... }, ... },
    "tdd-2": { "name": "tdd", "source": { "type": "github", "repo": "backnotprop/pstack", ... }, ... }
  }
}
```

- `name` is optional when the lock is read. A missing `name` means `name = key`. Thus a
  current lock file is a valid new lock file, and no migration step writes to disk.
- `version` stays `1`. An older app that reads a new lock sees `tdd-2` as a skill name,
  reports `name_mismatch`, and installs into `<root>/tdd-2`. Data is not lost. Downgrade
  is not supported beyond this.

## Import

An import adds a new record or replaces an existing record. The source identity decides
which one, not the name.

Source identity:

| Source   | Identity                                           |
| -------- | -------------------------------------------------- |
| `github` | `repo` (ignore case) + `skillPath`                 |
| `folder` | canonical `path`                                   |
| `zip`    | none: match by name among `zip` and `none` records |
| `none`   | none: match by name among `zip` and `none` records |

`ImportItem` gets an optional `replaceId`:

- `replaceId: Some(id)`: replace that record. The installs stay, and copy targets that are
  in sync receive the new content (this is the current `overwrite` behavior).
- `replaceId: None`: create a new record with a new id.

`detect_*` results get `matches: { id, sameSource: bool }[]`: the records with the same
name, and whether each one has the same source identity. The caller selects:

- Same source found: default is replace. Replace is allowed only with `overwrite`, as
  today, because it discards the old hub copy (the old copy goes to `.trash`).
- Only a different source with the same name: default is "add as new". This does not
  block the import. The UI shows the id that the new skill will get.

Inside one import batch, two items with the same name and no `replaceId` get two ids.

`ImportOutcome` gets `id`.

## Install

The target path is still `<agent root>/<name>`. A new blocker is added to `install_skill`:

- The target path is an install of a different record (`occupied_by: <id>`). Without
  `force`, the action returns `applied: false` with this blocker.
- With `force`, the install record moves from the old record to the new record, and the
  target gets the new content. A modified copy target of the old record goes to `.trash`
  first. This is the same rule as an instruction install over another template.

The current "foreign directory" blocker (an untracked directory at the target path) does
not change.

## Hub listing and drift

- `HubSkillView` gets `id`. `name` keeps its meaning.
- `probe_hub`, `diff_service`, `drift_service` and `install_service` take an id and use
  `env.hub_dir(id)`. The install directory name comes from `record.name`.
- `adopt_untracked_hub_dirs`: a hub directory without a record becomes a record with
  `id = directory name` and `name` from the frontmatter (else the directory name).

## Scan

`scan_folder` finds a skill directory by name and must select one record:

1. A symlink that points into the hub: the record of the link target directory.
2. The directory path is an install path of a record: that record.
3. A record with the same name and the same hash: that record (`identical`).
4. One record with the same name: that record (`different`).
5. More than one record with the same name: the record whose id is equal to the name,
   else the oldest one by `importedAt` (`different`).
6. No record: `new`.

`ScanItem` and `ScanDecision` get `id` (`None` for `new`). `import_scanned` keeps its
rule "the first folder per name becomes the hub copy". Scan does not create a second
record with the same name, because copies in agent directories are usually copies of one
skill. To add a second skill, the user imports it explicitly.

## Source updates

`check_source_updates` keys its candidates by id. The marketplace query still sends names
(`fetch_skills_by_names`), and the answer is matched by name and repo, as today. The
`names` argument becomes `ids`.

## Command surface

Tauri commands that take a skill `name` take `id`: `get_hub_skill`, `remove_hub_skill`,
`sync_skill`, `diff_skill`, `check_source_updates`, `install_skill`, `uninstall_skill`
(`InstallRequest.name` and `UninstallRequest.name` become `id`). These commands are
internal to the app, so no compatibility layer is added.

## CLI

Each command that takes a skill (`show`, `install`, `uninstall`, `remove`, `update`,
`sync`, `diff`) resolves the argument as follows:

1. An exact id match (ignore case) wins.
2. Else a unique name match.
3. Else, when more than one record has the name, exit 1 with
   `ambiguous skill 'tdd': tdd (mattpocock/skills), tdd-2 (backnotprop/pstack)`.

Rule 1 keeps current scripts working: every legacy record has `id = name`.

Other changes:

- `list` and `show`: an `ID` column, shown when an id is different from its name.
  `--json` gets `id` on each view. The change only adds fields.
- `import --overwrite`: replaces a record with the same source identity. When only the
  name is equal, the import adds a new record and prints the new id. The import does not
  fail.
- `install` on an occupied target: prints the `occupied_by` blocker and exits 2. `--force`
  moves the install.

The instruction commands already use the same "id or unique name" rule
(`commands/instructions.rs`). The skill commands use the same helper.

## Frontend

- `api/hub.ts`: views and payloads use `id`. `stores/hub.ts`: `hubSkillsById`. A name
  lookup returns a list.
- Route `/skills/hub/<id>`.
- Library list: when two skills have the same name, each row shows its source
  (`owner/repo` or folder name) as a second label.
- `ImportSkillModal`: the current `existingNames` check (it ignores case differently from
  the backend) is replaced by `matches` from detection. Same source: "update existing",
  which needs the overwrite checkbox. Different source: an informational line
  "added as a second skill `tdd` (id `tdd-2`)", which does not block confirm.
- `InstallSkillModal` and the scope views: a target that is occupied by another skill with
  the same name goes through `confirmForce`, as other blockers do.

## Migration

None on disk. A missing `name` is read as the key. The first write of a record writes
`name`. Hub directories, symlinks and install records do not change.

## Tests

Core (`youskill-core`):

- Import of the same name from two GitHub repositories gives two records (`tdd`,
  `tdd-2`) and two hub directories.
- Import from the same repository and `skillPath` replaces only with `overwrite`.
- Import of the same name with a different case, from a different source, gives a new id.
- Install of `tdd-2` into a root where `tdd` is installed: blocked with `occupied_by`.
  With force: the install record moves, and a modified old copy goes to `.trash`.
- Install of `tdd` (user scope) and `tdd-2` (project scope) at the same time: both apply.
- A lock without `name` fields lists the same as before.
- `name_mismatch` compares with `record.name`, not the key.
- Scan: matches by link target, by install path, by hash, and the tie rule.

CLI (`tests/cli.rs`):

- `show tdd` resolves the exact id. `show <name>` with two records and no id match
  exits 1 with the ambiguity message.
- `install` on an occupied target exits 2. With `--force`, it exits 0.

## Implementation steps

Each step compiles and passes `npm run check` and `npm run test`.

1. Model: `SkillRecord.name` (serde default from the key), id generation, `name_mismatch`
   against `record.name`. Services take an id; the install directory uses `record.name`.
   No change in behavior yet, because `id = name` everywhere.
2. Import: source identity, `replaceId`, `matches` in detection, new ids for same-name
   imports.
3. Install: the `occupied_by` blocker and the force transfer.
4. Scan and source updates: matching by id.
5. CLI: the id-or-name resolver, `ID` column, import and install messages.
6. Frontend: ids in the API, the store and the routes; import and install modals.
7. Docs and CHANGELOG (`Changed`: same-name skills from different sources;
   `--json` gets `id`).

## Open questions

- Should the user be able to choose the id at import (for example `tdd-pstack`)? The
  proposal generates it. A later `rename id` command would need a hub directory move and a
  relink of symlink installs.
- Should zip imports with the same name default to "add as new" instead of "replace"? The
  proposal uses name matching for sources without identity, so a repeated import of the
  same archive replaces and does not make duplicates.
