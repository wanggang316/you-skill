# Skill Identity

Status: implemented. This document is the design record of how the hub separates the
identity of a skill from its name. [ARCHITECTURE.md](ARCHITECTURE.md) and
[CLI.md](CLI.md) describe the current behavior.

The implementation differs from this design in these points:

- Detection results do not carry the matches. `plan_import(items)` returns an
  `ImportPlan` per import item (`matches`, `defaultReplaceId`, `replaceRequired`), because
  the source of an item is only known when the caller builds it.
- `InstallRequest.name` and `UninstallRequest.name` keep their name and take an id or a
  unique name. The Tauri commands that take one skill take `id`.
- `list` shows an `ID` column next to the `SOURCE` column.
- A failed relink during the migration is logged, not put in the migration report.

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
- Each skill has a stable id that does not depend on its name, its source or the import
  order. The id is the lock key and part of the hub path.
- Each source location is in the hub at most once. The source location is also the
  portable way to refer to a skill (CLI, scripts, other machines).
- The name stays the install directory name and the agent runtime name. The content of
  SKILL.md is not changed.
- Two skills with the same name cannot be installed into the same agent root at the same
  time. This check moves from import to install.
- Existing CLI scripts that use a skill name continue to work while the name is unique.

## Non-goals

- Rename on install (an install directory name that is different from the skill name).
  Agents read the frontmatter `name`, so a different directory name does not remove the
  conflict at runtime.
- Rename of a skill (a change of `record.name`). A pull that changes the frontmatter name
  gives `name_mismatch`, as today. A rename action can come later.
- Detection of shadowing across scopes (a user-level `tdd` and a project-level `tdd` that
  the same agent reads). The agent decides which one wins.
- Downgrade to an app version before this change (see [Lock file](#lock-file)).

## Model

Three values identify a skill. Each one has one role:

| Value       | Example                                     | Role                                       | Changes when          |
| ----------- | ------------------------------------------- | ------------------------------------------ | --------------------- |
| `id`        | `3f2a9c1e-...` (UUID v4)                    | lock key, hub path                         | never                 |
| `sourceKey` | `github:backnotprop/pstack@main:skills/tdd` | uniqueness of a source, portable reference | the source changes    |
| `name`      | `tdd`                                       | install directory, agent runtime name      | never (see non-goals) |

```
SkillRecord (key: id)
  name       install directory name and agent runtime name
  source     github | folder | zip | none
  hash, importedAt, updatedAt
  installs[] each install path is <agent root>/<name>
```

- The id is a UUID v4, as in the instruction library (`instruction_service::new_id`,
  `is_id`). The two libraries share one rule: a key is an id, and a unique name also
  resolves.
- `name` is stored in the record. `hub_state` is `name_mismatch` when the frontmatter
  name of the hub copy is different from `record.name` (today it compares with the key).
- `sourceKey` is computed from `source` and is not stored. See [Source key](#source-key).

### Hub layout

```
~/.youskill/skills/<id>/<name>/SKILL.md
```

The hub directory of a skill is `<id>/<name>`, not `<id>`:

- The Agent Skills specification requires that the frontmatter `name` is equal to the
  name of the directory that contains SKILL.md. A symlink install
  (`~/.claude/skills/tdd -> ~/.youskill/skills/<id>/tdd`) keeps this true also for an
  agent that resolves the symlink before it reads the skill.
- A path in a terminal or in Finder shows the skill name.

`env.hub_dir(id, name)` returns this path. Services get `name` from the record. The
`<id>` directory contains only the `<name>` directory.

## Source key

| Source   | `sourceKey`                                 | Normalization                                                                 |
| -------- | ------------------------------------------- | ----------------------------------------------------------------------------- |
| `github` | `github:<owner>/<repo>@<branch>:<skillDir>` | `owner/repo` in lower case; `skillDir` is `skillPath` without `/SKILL.md`     |
| `folder` | `folder:<path>`                             | canonical absolute path of the skill directory, for example `folder:/a/b/tdd` |
| `zip`    | `zip`                                       | type only                                                                     |
| `none`   | `none`                                      | type only                                                                     |

Every record has a key, and the key always starts with the source type. Thus the key
shows how a skill came into the hub, also for a zip import.

There are two kinds of key:

- A location key (`<type>:<location>`) is given to a source that YouSkill reads again
  after the import: `github` (`check_source_updates`, `pull_source`) and `folder`
  (`pull_source`, `push_source`). For these sources the location is the identity of the
  skill. A location key is unique and is a reference (see [CLI](#cli)).
- A type-only key (`zip`, `none`) only records the source type. It is not unique and is
  not a reference. A `zip` source is read once: the archive path records where the
  content came from, but it is not an identity, because the same archive can be at a
  different path and a different archive can be at the same path. `none` has no known
  origin (legacy migration, `adopt_untracked_hub_dirs`, scan). An import never matches a
  record by a type-only key. See [Import](#import).

### Branch

The branch is part of the key. One skill on `main` and on `next` gives two records,
because the content and the update stream are different. The two records have the same
name, so they cannot be installed into the same agent root (see [Install](#install)), but
each one can be installed into a different scope.

- The key is unambiguous: an owner and a repository name contain no `@`, and a Git branch
  name contains no `:`. A branch name can contain `/` and `@`
  (`github:o/r@feat/x:skills/tdd`).
- A new import always has a resolved branch: `GithubHelper::clone_repo_to` returns the
  branch that it downloaded.
- A legacy record can have no branch (`branch: None`). Its key is
  `github:<owner>/<repo>:<skillDir>`. An import that resolves the default branch of that
  repository matches such a record, and the replace writes the branch into the record.
  Thus a legacy record does not get a duplicate.

### Rules

- No two records have the same location key. An import with a known location key
  replaces that record and does not add a new one.
- When a source changes (`pull_source` with an updated source, a repository rename), the
  key is computed again. The id and the hub path do not change. A change to a key that
  another record has is refused.

## Lock file

```json
{
  "version": 2,
  "skills": {
    "3f2a9c1e-...": { "name": "tdd", "source": { "type": "github", "repo": "mattpocock/skills", ... }, ... },
    "8b71d0f4-...": { "name": "tdd", "source": { "type": "github", "repo": "backnotprop/pstack", ... }, ... }
  }
}
```

- `version` is `2`. A lock with a key that is not a UUID is a version 1 lock and goes
  through the [migration](#migration).
- Downgrade is not supported. An older app reads each UUID as a skill name and finds no
  SKILL.md at `skills/<id>/`, so it shows each skill as `invalid`. The older app does not
  check `version`, so this cannot be prevented.

## Import

An import adds a new record or replaces an existing record. For a source with a location
key, the key decides. For a source with a type-only key, the import proposes a default and
the user confirms it.

`ImportItem` gets an optional `replaceId`:

- `replaceId: Some(id)`: replace that record. The installs stay, and copy targets that are
  in sync receive the new content (this is the current `overwrite` behavior).
- `replaceId: None`: create a new record with a new id.

Every replace needs `overwrite`, as today, because it discards the old hub copy (the old
copy goes to `.trash`).

`plan_import` returns `matches: { id, sameSource: bool }[]`: the records with the same
location key (`sameSource: true`), and the records with the same name and a different
location key or a type-only key (`sameSource: false`). It also returns `defaultReplaceId`:

| Detected source | Existing record with the same name     | `defaultReplaceId`            | Can change |
| --------------- | -------------------------------------- | ----------------------------- | ---------- |
| `github`        | same location key                      | that record                   | no         |
| `folder`        | same location key                      | that record                   | no         |
| `zip`           | key `zip` or `none`                    | the one updated most recently | yes        |
| any             | only records with another location key | none (add as new)             | yes        |

- Same location key: replace, and `import_one` refuses `replaceId: None`. Two records with
  one location key cannot exist.
- `zip`: a new version of an archive is the usual case, so a same-name `zip` or `none`
  record is replaced by default. The user can select another match or "add as new".
  When more than one such record exists, the default is the one with the latest
  `updatedAt`, and the UI lists the others.
- Different location key: add as new by default. The user can select a match to replace
  instead.

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

- `HubSkillView` gets `id` and `sourceKey`. `name` keeps its meaning.
- `probe_hub`, `diff_service`, `drift_service` and `install_service` take an id and use
  `env.hub_dir(id, record.name)`. The install directory name comes from `record.name`.
- `trash_dir` keeps `.trash/<name>-<stamp>`.
- `adopt_untracked_hub_dirs` accepts two forms:
  - `skills/<uuid>/<name>/SKILL.md` without a record: a record with that id. This also
    recovers an interrupted migration.
  - `skills/<name>/SKILL.md` (a directory added by hand): a new id, and the directory
    moves to `skills/<id>/<name>`.

## Scan

`scan_folder` finds a skill directory by name and must select one record:

1. A symlink that points into the hub: the record of the link target.
2. The directory path is an install path of a record: that record.
3. A record with the same name and the same hash: that record (`identical`).
4. One record with the same name: that record (`different`).
5. More than one record with the same name: the oldest one by `importedAt`
   (`different`).
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
`sync`, `diff`) resolves the argument in this order:

1. An id.
2. A location key, or a part of one that matches one record. The `github:` prefix and
   `@<branch>` are optional, and `<owner>/<repo>:<name>` matches by name inside that
   repository. Examples: `github:backnotprop/pstack@main:skills/tdd`,
   `backnotprop/pstack:skills/tdd`, `backnotprop/pstack@next:tdd`, `backnotprop/pstack:tdd`.
3. A unique name.
4. More than one match in step 2 or 3: exit 1 and list each candidate with its id and source key:
   `ambiguous skill 'tdd': 3f2a9c1e (github:mattpocock/skills@main:skills/tdd), 8b71d0f4 (zip)`.

Rule 3 keeps current scripts working while a name is unique. Scripts that must work on
more than one machine use a location key, not an id.

Other changes:

- `list` and `show`: a `SOURCE` column with the source key. `--json` gets `id` and
  `sourceKey` on each view. The change only adds fields.
- `import` uses `defaultReplaceId`. When it is set, the import needs `--overwrite`, else
  it exits 1 and names the record, as today. When it is not set, the import adds a new
  record and prints its id.
- `import <zip> --replace <skill>`: replaces the given record (any reference form from
  the list above) instead of the default. It needs `--pick` when the archive has more
  than one skill, and it needs `--overwrite`.
- `import <zip> --add`: adds a new record also when a default replace exists.
- `install` on an occupied target: prints the `occupied_by` blocker and exits 2. `--force`
  moves the install.

The resolver is shared with the instruction commands (`commands/instructions.rs`), which
already accept an id or a unique name.

## Frontend

- `api/hub.ts`: views and payloads use `id`. `stores/hub.ts`: `hubSkillsById`. A name
  lookup returns a list.
- Route `/skills/hub/<id>`.
- Library list: when two skills have the same name, each row shows its source
  (`owner/repo@branch`, the folder name, the archive name or the import date) as a second
  label.
- `ImportSkillModal`: the current `existingNames` check (it compares case differently
  from the backend) is replaced by `matches` from detection. Same source: "update
  existing", which needs the overwrite checkbox. A zip skill with a same-name `zip` or
  `none` record: "replace `tdd`" is selected by default and needs the overwrite
  checkbox; the user can select another match or "add as new". Same name, different
  location key: "add as a second skill `tdd`" by default, with an option to replace one
  of the matches.
- `InstallSkillModal` and the scope views: a target that is occupied by another skill with
  the same name goes through `confirmForce`, as other blockers do.

## Migration

The migration runs on the first listing, after `ensure_migrated`, under `ops_guard`, as
`instruction_service::migrate_named_records` does. For each record whose key is not a
UUID:

1. Make a new id. Create `skills/<id>/` and move `skills/<key>` to `skills/<id>/<key>`.
2. Relink each symlink install of the record that points to `skills/<key>` to the new
   path. Copy installs do not change.
3. Write the lock: the record moves from `<key>` to `<id>`, and `name` is set to `<key>`.
   Set `version` to `2`.

Each record is migrated alone, so an interrupted run continues on the next listing:

- Interrupted after step 1: the lock still has `<key>`, and `skills/<key>` does not exist.
  The migration finds `skills/<uuid>/<key>` that has no record, uses that id, and
  continues at step 2.
- Interrupted after step 2: step 2 is idempotent.

A relink failure is logged and does not stop the migration. A
symlink that is not in the lock and points to `skills/<key>` breaks. A scan shows it as
`broken_link`.

## Tests

Core (`youskill-core`):

- Import of the same name from two GitHub repositories gives two records, two ids and two
  hub directories `skills/<id>/tdd`.
- Import with a known location key replaces only with `overwrite`, and never adds a record.
- Import of one skill from two branches gives two records.
- Import from the default branch matches a legacy record without a branch, and writes the
  branch into it.
- A zip import with a same-name `zip` record proposes that record as `defaultReplaceId`;
  with two such records, the one updated most recently. A replace without `overwrite` is
  refused. With `replaceId: None`, the import adds a record.
- A zip import with only a same-name `github` record proposes no replace.
- A source change to a key that another record has is refused.
- `zip` and `none` keys are not resolved as references, and two zip records can have the
  same name and the same key.
- Install of the second `tdd` into a root where the first `tdd` is installed: blocked with
  `occupied_by`. With force: the install record moves, and a modified old copy goes to
  `.trash`.
- Install of one `tdd` at user scope and the other `tdd` at project scope: both apply.
- `name_mismatch` compares with `record.name`.
- Migration: a version 1 lock with symlink and copy installs gives UUID keys, the new
  layout and relinked symlinks. A second run does nothing. A run stopped after step 1
  completes on the next run.
- `adopt_untracked_hub_dirs`: both forms.
- Scan: matches by link target, by install path, by hash, and the tie rule.

CLI (`tests/cli.rs`):

- A skill resolves by id, by location key, by a key without branch, by
  `<owner>/<repo>:<name>` and by unique name. A key without branch that matches two
  branches exits 1.
- `show tdd` with two records exits 1 with the ambiguity message.
- `install` on an occupied target exits 2. With `--force`, it exits 0.

## Implementation steps

Each step compiles and passes `npm run check` and `npm run test`.

1. Model and layout: `SkillRecord.name`, UUID ids, `env.hub_dir(id, name)`, the
   migration, `adopt_untracked_hub_dirs`, `name_mismatch` against `record.name`.
   Services take an id or a unique name (`hub_service::resolve_skill`), so the app and
   the CLI keep working with names until steps 5 and 6. Behavior is the same except for
   the layout.
2. Source key and import: `sourceKey`, `replaceId`, `matches` in detection, the
   uniqueness rule, the legacy branch match.
3. Install: the `occupied_by` blocker and the force transfer.
4. Scan and source updates: matching by id.
5. CLI: the shared resolver, the `SOURCE` column, import and install messages.
6. Frontend: ids in the API, the store and the routes; import and install modals.
7. Docs and CHANGELOG (`Breaking Changes`: hub layout and lock version 2, no downgrade;
   `Changed`: same-name skills from different sources; `--json` gets `id` and
   `sourceKey`).
