//! Auto sync: one library sync cycle that the desktop app starts on a schedule
//! (`docs/decisions/0005-library-auto-sync.md`).
//!
//! A cycle plans a restore of the branch head, applies it with the sync policy when no item
//! needs a person, then backs up. It never runs installs or `apply_pending`, and it never
//! takes a deferred item. A state that needs a person stops the cycle before it writes.

use super::*;
use crate::services::lock_service::ops_busy;

/// What a cycle does with a repository change (`take_remote`, an incoming item).
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryChangePolicy {
  #[default]
  Apply,
  Ask,
}

/// What a cycle does with a removal candidate, also one another device made device-only.
/// There is no "keep": a kept removal is committed back and reverts the other device.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RemovalPolicy {
  Remove,
  #[default]
  Ask,
}

/// What a cycle does with a conflict.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
  #[default]
  Ask,
  KeepLocal,
  TakeRemote,
  KeepBoth,
}

/// For each kind of restore plan item, the automatic action or "ask". A setting of this
/// computer.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncPolicy {
  pub repository_change: RepositoryChangePolicy,
  pub removal: RemovalPolicy,
  pub conflict: ConflictPolicy,
}

/// The steps of a cycle and the policy of its restore.
#[derive(Debug, Clone, Copy)]
pub struct AutoSyncOptions {
  pub restore: bool,
  pub backup: bool,
  pub policy: SyncPolicy,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutoSyncResult {
  /// Nothing to restore or back up.
  UpToDate,
  /// The cycle restored, backed up, or both.
  Synced,
  /// A person must sync manually; see `reason`.
  NeedsAttention,
  /// Another operation held the lock, or the repository moved during the restore. The next
  /// interval tries again.
  Skipped,
  /// The library is not connected to a repository.
  NotConnected,
}

/// Why a cycle needs a person.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttentionReason {
  /// Plan items with the policy "ask" (or a choice that cannot apply to them).
  Ask,
  ChooseMode,
  MigrationRequired,
  RemoteNotEmpty,
  Diverged,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutoSyncOutcome {
  pub result: AutoSyncResult,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reason: Option<AttentionReason>,
  /// Keys of the plan items that wait for a decision.
  #[serde(default)]
  pub ask: Vec<String>,
  /// The restore changed the hub or the lock files.
  pub restored: bool,
  /// The backup pushed a commit.
  pub backed_up: bool,
  /// The custom agent definitions changed; agent lists need a refresh.
  pub agents_changed: bool,
}

impl AutoSyncOutcome {
  fn new(result: AutoSyncResult) -> Self {
    Self {
      result,
      reason: None,
      ask: Vec::new(),
      restored: false,
      backed_up: false,
      agents_changed: false,
    }
  }

  fn attention(reason: AttentionReason) -> Self {
    Self {
      reason: Some(reason),
      ..Self::new(AutoSyncResult::NeedsAttention)
    }
  }

  fn finish(mut self) -> Self {
    self.result = if self.reason.is_some() {
      AutoSyncResult::NeedsAttention
    } else if self.restored || self.backed_up {
      AutoSyncResult::Synced
    } else {
      AutoSyncResult::UpToDate
    };
    self
  }
}

/// Map a ready plan to restore decisions with `policy`. Returns the keys of the items that
/// need a person instead when there is one: the whole restore then waits, because the API
/// cannot leave a conflict out and an undecided removal is kept and committed back.
pub fn decisions_for(
  plan: &RestorePlan,
  policy: &SyncPolicy,
) -> Result<RestoreDecisions, Vec<String>> {
  let mut decisions = RestoreDecisions::default();
  let mut ask = Vec::new();
  for item in &plan.items {
    let key = item.key.clone();
    if item.deferred {
      // The user left this version out on purpose.
      if item.resolution == Resolution::TakeRemote {
        decisions.include.insert(key, false);
      }
      continue;
    }
    match item.resolution {
      Resolution::KeepLocal => {},
      Resolution::TakeRemote => match policy.repository_change {
        RepositoryChangePolicy::Apply => {
          decisions.include.insert(key, true);
        },
        RepositoryChangePolicy::Ask => ask.push(key),
      },
      Resolution::RemovalCandidate => match policy.removal {
        RemovalPolicy::Remove => {
          decisions.removals.insert(key, RemovalChoice::Remove);
        },
        RemovalPolicy::Ask => ask.push(key),
      },
      Resolution::Conflict => {
        let choice = match policy.conflict {
          ConflictPolicy::Ask => None,
          ConflictPolicy::KeepLocal => Some(ConflictChoice::KeepLocal),
          ConflictPolicy::TakeRemote => Some(ConflictChoice::TakeRemote),
          // Only an item that exists on both sides, and not a custom agent, can be kept twice.
          ConflictPolicy::KeepBoth => Some(ConflictChoice::KeepBoth).filter(|_| {
            item.kind != ItemKind::Agent
              && item.local_change != ItemChange::Removed
              && item.remote_change != ItemChange::Removed
          }),
        };
        match choice {
          Some(choice) => {
            decisions.conflicts.insert(key, choice);
          },
          None => ask.push(key),
        }
      },
    }
  }
  if ask.is_empty() {
    Ok(decisions)
  } else {
    Err(ask)
  }
}

pub fn auto_sync(env: &Env, options: &AutoSyncOptions) -> Result<AutoSyncOutcome, String> {
  if read_state(env)?.connected().is_err() {
    return Ok(AutoSyncOutcome::new(AutoSyncResult::NotConnected));
  }
  let remote = github_remote(env)?;
  auto_sync_with(env, &remote, options)
}

/// Run one cycle: restore the branch head with the sync policy, then back up. A plan that
/// needs a first-connect choice or a migration stops the cycle before it writes; plan items
/// that need a person stop only the restore. A stale restore ends the cycle.
pub fn auto_sync_with(
  env: &Env,
  remote: &dyn RemoteRepo,
  options: &AutoSyncOptions,
) -> Result<AutoSyncOutcome, String> {
  if read_state(env)?.connected().is_err() {
    return Ok(AutoSyncOutcome::new(AutoSyncResult::NotConnected));
  }
  // A cycle does not wait for a manual operation; the next interval tries again.
  if ops_busy(env)? {
    return Ok(AutoSyncOutcome::new(AutoSyncResult::Skipped));
  }
  let mut outcome = AutoSyncOutcome::new(AutoSyncResult::UpToDate);

  if options.restore {
    let plan = plan_restore_with(env, remote, RestoreMode::Pull, None)?;
    match plan.state {
      PlanState::ChooseMode => return Ok(AutoSyncOutcome::attention(AttentionReason::ChooseMode)),
      PlanState::MigrationRequired => {
        return Ok(AutoSyncOutcome::attention(
          AttentionReason::MigrationRequired,
        ))
      },
      PlanState::UpToDate | PlanState::RemoteEmpty => {},
      PlanState::Ready => match decisions_for(&plan, &options.policy) {
        Err(ask) => {
          outcome.reason = Some(AttentionReason::Ask);
          outcome.ask = ask;
        },
        Ok(decisions) => {
          let writes = !decisions.conflicts.is_empty()
            || !decisions.removals.is_empty()
            || decisions.include.values().any(|include| *include);
          // Only local changes and deferred items: the backup handles them, and a restore
          // would replace the checkpoint for nothing. A moved head still needs the restore
          // to become the base.
          let moved = read_state(env)?.last_synced_commit != plan.remote_head;
          if !writes && !moved {
            return backup_step(env, remote, options, outcome);
          }
          // No installs run, so the install mode is not used.
          let restored = apply_restore_with(
            env,
            remote,
            &plan,
            &decisions,
            &RestoreOptions::default(),
            InstallMode::Copy,
          )?;
          outcome.restored = restored.local_applied && writes;
          outcome.agents_changed = restored.agents_changed;
          if restored.result == RestoreResult::Stale {
            if outcome.restored {
              return Ok(outcome.finish());
            }
            return Ok(AutoSyncOutcome::new(AutoSyncResult::Skipped));
          }
        },
      },
    }
  }
  backup_step(env, remote, options, outcome)
}

fn backup_step(
  env: &Env,
  remote: &dyn RemoteRepo,
  options: &AutoSyncOptions,
  mut outcome: AutoSyncOutcome,
) -> Result<AutoSyncOutcome, String> {
  if options.backup {
    let backup = backup_with(env, remote)?;
    let reason = match backup.result {
      BackupResult::Pushed => {
        outcome.backed_up = true;
        None
      },
      // A computer that only backs up leaves the repository changes for a manual restore.
      BackupResult::UpToDate | BackupResult::RemoteAhead => None,
      BackupResult::Diverged => Some(AttentionReason::Diverged),
      BackupResult::RemoteNotEmpty => Some(AttentionReason::RemoteNotEmpty),
      BackupResult::MigrationRequired => Some(AttentionReason::MigrationRequired),
    };
    if outcome.reason.is_none() {
      outcome.reason = reason;
    }
  }
  Ok(outcome.finish())
}
