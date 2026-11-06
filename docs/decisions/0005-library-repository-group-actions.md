# Library repository group actions

Status: Accepted

## Context

The Library groups hub skills by their GitHub repository (group key `github:<owner/repo>`, without the branch). To add or remove skills of one repository, the user must open the import dialog and type the repository again. To install, update or remove all skills of a repository, the user must select each skill with Shift.

The app already has these flows: the import dialog with `planImport` (it replaces skills that are already in the Library), the install dialog with `replace: true`, the remove dialog (it can also uninstall installed copies), `checkSourceUpdates`, and the `pull_source` sync (it returns blockers when it would overwrite local changes, and needs force to continue).

## Decision

- Add a `...` menu, "Repository actions" (仓库操作), to each repository group. Folder groups and groups without a source do not get the menu for now. Make the menu component reusable for other group kinds.
- The menu has four items. Each item uses the existing flow with the whole group as its scope.
- **Import**: open the GitHub tab of the import dialog with the repository filled in, and start detection. If all skills of the group come from one branch, use that branch. Otherwise use the default branch. Skills that are already in the Library show as imported and are not selected. When the user selects one, the existing replace flow applies. Import only adds or replaces skills. A skill that is not selected is not removed.
- **Install**: select all skills of the group and open the install dialog with `replace: true`, the same as a Shift selection.
- **Update**:
  1. Do a forced update check for all skills of the group. The check sends one request for each `(repo, branch)`.
  2. Before any download, find the skills with the state `update_available` whose hub copy has local changes (`probe_hub` gives `Modified` or `NameMismatch`). If there are such skills, ask for force one time for all of them: overwrite all, or skip all.
  3. A batch backend command pulls the selected skills. It downloads and extracts each `(repo, branch)` one time, and runs `pull_from_dir` for each skill from that extracted tree. The remote sha comes from the `latest_remote_sha` that step 1 stored, not from one request for each skill.
  4. Show a report: the number updated, the number already up to date, and each skipped or failed skill. If a file changes between step 2 and the write, `pull_from_dir` blocks that skill and the report shows it as skipped.
  - Do not call the single-skill `pull_source` in a loop. It downloads the full repository archive for each skill, so a group of N skills downloads the same repository N times.
  - Update does not push to install targets, the same as `pull_source` for one skill. Targets that become `outdated` show in the drift panel.
- **Remove**: put all skills of the group in the existing remove dialog. It shows the count and keeps the option to also uninstall installed copies.
- To act on part of a group, the user continues to use Shift selection.

## Consequences

- Remove is a separate action with its own confirmation, because it can also uninstall copies from agent directories. The import list cannot add and remove skills in one step.
- After an update, the user may need one more step to push the new version to install targets.
- A partial install or remove cannot start from the menu.

## Reconsider when

- Users often push to targets right after a group update. Then add an option to push to targets automatically.
- Folder groups need the same actions. Then add a menu with the scan semantics of folders.
