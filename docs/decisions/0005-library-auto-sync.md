# Library sync: auto sync and sync policy

Status: Accepted

## Context

Library sync starts only from a user action: a backup, or a restore with a preview, a checkpoint and undo ([#60](https://github.com/wanggang316/youskill/issues/60), [ADR 0004](0004-library-sync-item-scope.md)). A library that changes on two devices stays apart until the user syncs on each device. Users want the app to back up and restore on a schedule.

A restore writes the hub, the lock files and the sync state of this computer. Some plan items need a person by the current rules:

- `RestoreDecisions.conflicts` must have a choice for every conflict. The API cannot apply a plan and leave one conflict out.
- A removal candidate without a decision is kept, and a kept removal is committed on top of the head. Thus an empty decision puts an item back that another device removed.

## Decision

### Schedule

- Auto sync is a setting of this computer. It is not synced. It is off by default, so an app update does not write to a library without the user's consent.
- Auto backup and auto restore are two switches. Both are on when the user turns auto sync on.
- Interval: 15 minutes, 1 hour (default), 6 hours or 24 hours. The app also syncs once at start. The timer continues while the window is hidden in the tray. A local change does not start a sync.
- Only the desktop app runs auto sync. The CLI does not.
- A cycle uses the operation lock. The app skips the cycle while a manual sync operation runs or while the restore dialog is open. A failed cycle waits for the next interval. It does not retry before that.

### Cycle

1. `plan_restore(pull)` on the branch head.
2. If auto restore is on, apply the plan with the sync policy (below).
3. If auto backup is on, `backup`.

The cycle stops and writes nothing when the plan state or the backup result needs a person: `choose_mode`, `migration_required`, `remote_not_empty` or `diverged`. The user resolves it with a manual sync.

### Sync policy

The sync policy gives, for each kind of plan item, what a cycle does with it. It is a setting of this computer.

| Kind of item | Choices | Default |
| --- | --- | --- |
| Repository change (`take_remote`, incoming item) | apply, ask | apply |
| Removal candidate (also `madeDeviceOnly`) | remove, ask | ask |
| Conflict | ask, keep local, take repository, keep both | ask |

- A cycle never includes a deferred item. The user left it out of a restore on purpose.
- Removal candidates have no "keep" choice. A kept removal is committed back to the repository, so an automatic keep reverts the removal of another device.
- When one item of the plan gets "ask", the cycle does not restore. It still tries the backup, and it shows that a manual restore is necessary.
- An automatic removal uses the normal hub and instruction removal, the same as a confirmed removal in a manual restore.
- An auto restore does not run installs (`applyInstalls` is off) and does not run `apply_pending`. Copy installs of a changed item show as outdated.
- An auto restore that writes takes the checkpoint, as a manual restore does. It replaces the earlier checkpoint. Settings offers undo of the last auto restore.

### Feedback

- A successful cycle shows nothing. The library sync section in Settings shows the time and the result of the last auto sync.
- When a cycle needs a person (an "ask" item, or a stop state above) or fails, the Settings navigation item shows a badge. The app does not send a system notification.

## Consequences

- Library sync can change the hub and the lock files without a user action. Every automatic write has a checkpoint and can be undone.
- One "ask" item stops all repository changes from arriving until the user restores. The badge makes this visible.
- An automatic restore replaces the checkpoint of an earlier manual restore. Undo then goes back only to the state before the last auto restore.
- Each cycle makes GitHub API requests, also when nothing changed. The shortest interval is 15 minutes to keep this low.
- Two devices with "keep local" on conflicts can overwrite each other's edits in turn. The default "ask" prevents this.

## Reconsider when

- Users often see restores stop because of one "ask" item. Then add a defer state for conflicts and removal candidates, so that the rest of the plan applies.
- Users want a sync after each local change instead of on a schedule.
- The CLI gets a daemon mode.
