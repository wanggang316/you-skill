//! Auto sync cycles on the fake repository.

use super::*;

fn options(policy: SyncPolicy) -> AutoSyncOptions {
  AutoSyncOptions {
    restore: true,
    backup: true,
    policy,
  }
}

fn cycle(env: &Env, repo: &FakeRepo, options: AutoSyncOptions) -> AutoSyncOutcome {
  auto_sync_with(env, repo, &options).unwrap()
}

fn default_cycle(env: &Env, repo: &FakeRepo) -> AutoSyncOutcome {
  cycle(env, repo, options(SyncPolicy::default()))
}

fn assert_attention(outcome: &AutoSyncOutcome, reason: AttentionReason) {
  assert_eq!(
    outcome.result,
    AutoSyncResult::NeedsAttention,
    "{:?}",
    outcome
  );
  assert_eq!(outcome.reason, Some(reason), "{:?}", outcome);
  assert!(!outcome.restored, "{:?}", outcome);
}

fn sync_files(env: &Env) -> [Option<Vec<u8>>; 3] {
  [
    env.lock_path.clone(),
    env.instruction_lock_path.clone(),
    state_path(env),
  ]
  .map(|path| fs::read(path).ok())
}

#[test]
fn policy_defaults_apply_changes_and_ask_for_removals_and_conflicts() {
  let policy: SyncPolicy = serde_json::from_str("{}").unwrap();
  assert_eq!(policy.repository_change, RepositoryChangePolicy::Apply);
  assert_eq!(policy.removal, RemovalPolicy::Ask);
  assert_eq!(policy.conflict, ConflictPolicy::Ask);
  assert_eq!(
    serde_json::to_value(SyncPolicy::default()).unwrap(),
    serde_json::json!({ "repositoryChange": "apply", "removal": "ask", "conflict": "ask" })
  );
}

#[test]
fn a_synced_library_is_up_to_date() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (_a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  let commits = repo.commit_count();
  let outcome = default_cycle(&b, &repo);
  assert_eq!(outcome.result, AutoSyncResult::UpToDate, "{:?}", outcome);
  assert_eq!(repo.commit_count(), commits);
}

#[test]
fn repository_changes_apply_by_default_without_installs() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  install(&b, FOO, &["claude-code"]);
  backup_with(&b, &repo).unwrap();
  pull(&a, &repo);
  edit_skill(&a, FOO, "foo", "v2");
  add_skill(&a, BAR, "bar", "v1");
  install(&a, BAR, &["claude-code"]);
  backup_with(&a, &repo).unwrap();

  let outcome = default_cycle(&b, &repo);
  assert_eq!(outcome.result, AutoSyncResult::Synced, "{:?}", outcome);
  assert!(outcome.restored);
  assert!(skill_body(&b, FOO, "foo").contains("v2"));
  assert!(store(&b).read().unwrap().skills.contains_key(BAR));
  // The copy install of the changed skill is not updated, and nothing new is installed.
  let installed = b.home.join(".claude-code/skills/foo/SKILL.md");
  assert!(fs::read_to_string(installed).unwrap().contains("v1"));
  assert!(!b.home.join(".claude-code/skills/bar").exists());
  assert!(status(&b).unwrap().checkpoint.is_some());
  assert_eq!(default_cycle(&b, &repo).result, AutoSyncResult::UpToDate);
}

#[test]
fn a_repository_change_with_ask_stops_the_restore() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  edit_skill(&a, FOO, "foo", "v2");
  backup_with(&a, &repo).unwrap();
  let before = sync_files(&b);
  let checkpoint = status(&b).unwrap().checkpoint;
  let commits = repo.commit_count();

  let outcome = cycle(
    &b,
    &repo,
    options(SyncPolicy {
      repository_change: RepositoryChangePolicy::Ask,
      ..Default::default()
    }),
  );
  assert_attention(&outcome, AttentionReason::Ask);
  assert_eq!(outcome.ask, vec![skill_key(FOO)]);
  assert_eq!(sync_files(&b), before);
  assert_eq!(repo.commit_count(), commits);
  assert_eq!(status(&b).unwrap().checkpoint, checkpoint);
}

#[test]
fn a_removal_with_ask_stops_the_restore_and_the_backup_keeps_the_removal() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| {
    add_skill(a, FOO, "foo", "v1");
    add_skill(a, BAR, "bar", "v1");
  });
  hub_service::remove_hub_skill(&a, FOO, false).unwrap();
  backup_with(&a, &repo).unwrap();
  edit_skill(&b, BAR, "bar", "from b");

  let outcome = default_cycle(&b, &repo);
  assert_attention(&outcome, AttentionReason::Ask);
  assert_eq!(outcome.ask, vec![skill_key(FOO)]);
  assert!(outcome.backed_up, "the backup still runs");
  assert!(store(&b).read().unwrap().skills.contains_key(FOO));
  let library = library_at_head(&repo);
  assert!(
    !library.skills.contains_key(FOO),
    "the removal is not reverted"
  );
  assert!(String::from_utf8(
    repo
      .head_file(&format!("skills/{}/bar/SKILL.md", BAR))
      .unwrap()
  )
  .unwrap()
  .contains("from b"));
}

#[test]
fn a_removal_with_remove_deletes_the_item_and_undo_brings_it_back() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  hub_service::remove_hub_skill(&a, FOO, false).unwrap();
  backup_with(&a, &repo).unwrap();
  let before = sync_files(&b);

  let outcome = cycle(
    &b,
    &repo,
    options(SyncPolicy {
      removal: RemovalPolicy::Remove,
      ..Default::default()
    }),
  );
  assert_eq!(outcome.result, AutoSyncResult::Synced, "{:?}", outcome);
  assert!(outcome.restored);
  assert!(!store(&b).read().unwrap().skills.contains_key(FOO));
  assert!(!library_at_head(&repo).skills.contains_key(FOO));

  undo_restore(&b).unwrap();
  assert_eq!(sync_files(&b), before);
  assert!(store(&b).read().unwrap().skills.contains_key(FOO));
}

#[test]
fn a_device_only_removal_with_ask_stops_the_restore() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  make_device_only(&a, &skill_key(FOO));
  backup_with(&a, &repo).unwrap();

  let outcome = default_cycle(&b, &repo);
  assert_attention(&outcome, AttentionReason::Ask);
  assert!(store(&b).read().unwrap().skills.contains_key(FOO));
}

/// A and B edit FOO; A backs up first. Returns B.
fn conflict(root: &Path, repo: &FakeRepo) -> Env {
  let (a, b) = synced_pair(root, repo, |a| add_skill(a, FOO, "foo", "v1"));
  edit_skill(&a, FOO, "foo", "from a");
  backup_with(&a, repo).unwrap();
  edit_skill(&b, FOO, "foo", "from b");
  b
}

fn with_conflict(conflict: ConflictPolicy) -> AutoSyncOptions {
  options(SyncPolicy {
    conflict,
    ..Default::default()
  })
}

fn head_skill(repo: &FakeRepo, id: &str, name: &str) -> String {
  String::from_utf8(
    repo
      .head_file(&format!("skills/{}/{}/SKILL.md", id, name))
      .unwrap(),
  )
  .unwrap()
}

#[test]
fn a_conflict_with_ask_stops_the_restore_and_the_backup() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let b = conflict(tmp.path(), &repo);
  let commits = repo.commit_count();
  let outcome = cycle(&b, &repo, with_conflict(ConflictPolicy::Ask));
  assert_attention(&outcome, AttentionReason::Ask);
  assert!(!outcome.backed_up);
  assert!(skill_body(&b, FOO, "foo").contains("from b"));
  assert_eq!(repo.commit_count(), commits);
}

#[test]
fn a_conflict_with_keep_local_pushes_this_version() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let b = conflict(tmp.path(), &repo);
  let outcome = cycle(&b, &repo, with_conflict(ConflictPolicy::KeepLocal));
  assert_eq!(outcome.result, AutoSyncResult::Synced, "{:?}", outcome);
  assert!(skill_body(&b, FOO, "foo").contains("from b"));
  assert!(head_skill(&repo, FOO, "foo").contains("from b"));
  assert!(status(&b).unwrap().changes.is_empty());
}

#[test]
fn a_conflict_with_take_remote_takes_the_repository_version() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let b = conflict(tmp.path(), &repo);
  let outcome = cycle(&b, &repo, with_conflict(ConflictPolicy::TakeRemote));
  assert_eq!(outcome.result, AutoSyncResult::Synced, "{:?}", outcome);
  assert!(skill_body(&b, FOO, "foo").contains("from a"));
  assert!(head_skill(&repo, FOO, "foo").contains("from a"));
}

#[test]
fn a_conflict_with_keep_both_keeps_a_local_copy() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let b = conflict(tmp.path(), &repo);
  let outcome = cycle(&b, &repo, with_conflict(ConflictPolicy::KeepBoth));
  assert_eq!(outcome.result, AutoSyncResult::Synced, "{:?}", outcome);
  assert!(skill_body(&b, FOO, "foo").contains("from a"));
  let copies = skills_named(&b, "foo-local");
  assert_eq!(copies.len(), 1, "{:?}", store(&b).read().unwrap().skills);
  assert!(skill_body(&b, &copies[0], "foo-local").contains("from b"));
  assert!(library_at_head(&repo).skills.contains_key(&copies[0]));
}

#[test]
fn keep_both_asks_for_a_conflict_that_cannot_be_kept_twice() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  hub_service::remove_hub_skill(&a, FOO, false).unwrap();
  backup_with(&a, &repo).unwrap();
  edit_skill(&b, FOO, "foo", "from b");

  let outcome = cycle(&b, &repo, with_conflict(ConflictPolicy::KeepBoth));
  assert_attention(&outcome, AttentionReason::Ask);
  assert_eq!(outcome.ask, vec![skill_key(FOO)]);
}

#[test]
fn a_deferred_item_is_never_applied() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  edit_skill(&a, FOO, "foo", "v2");
  backup_with(&a, &repo).unwrap();
  let planned = plan(&b, &repo, RestoreMode::Pull);
  apply(
    &b,
    &repo,
    &planned,
    RestoreDecisions {
      include: BTreeMap::from([(skill_key(FOO), false)]),
      ..Default::default()
    },
  );
  let checkpoint = status(&b).unwrap().checkpoint;

  let outcome = default_cycle(&b, &repo);
  assert_eq!(outcome.result, AutoSyncResult::UpToDate, "{:?}", outcome);
  assert!(skill_body(&b, FOO, "foo").contains("v1"));
  assert!(read_state(&b)
    .unwrap()
    .deferred
    .contains_key(&skill_key(FOO)));
  assert_eq!(
    status(&b).unwrap().checkpoint,
    checkpoint,
    "no restore ran, so the checkpoint of the manual restore stays"
  );

  // A later change elsewhere applies, the deferred item still does not.
  add_skill(&a, BAR, "bar", "v1");
  backup_with(&a, &repo).unwrap();
  let outcome = default_cycle(&b, &repo);
  assert_eq!(outcome.result, AutoSyncResult::Synced, "{:?}", outcome);
  assert!(store(&b).read().unwrap().skills.contains_key(BAR));
  assert!(skill_body(&b, FOO, "foo").contains("v1"));
}

#[test]
fn a_first_connect_with_two_libraries_waits_for_a_mode() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let a = computer(tmp.path(), "a", &["claude-code"]);
  add_skill(&a, FOO, "foo", "v1");
  backup_with(&a, &repo).unwrap();
  let b = computer(tmp.path(), "b", &["claude-code"]);
  add_skill(&b, BAR, "bar", "v1");
  let before = sync_files(&b);
  let commits = repo.commit_count();

  assert_attention(&default_cycle(&b, &repo), AttentionReason::ChooseMode);
  assert_eq!(sync_files(&b), before);
  assert_eq!(repo.commit_count(), commits);

  // Without the restore, the backup finds the library in the repository.
  let backup_only = cycle(
    &b,
    &repo,
    AutoSyncOptions {
      restore: false,
      ..options(SyncPolicy::default())
    },
  );
  assert_attention(&backup_only, AttentionReason::RemoteNotEmpty);
  assert_eq!(repo.commit_count(), commits);
}

#[test]
fn a_version_1_repository_waits_for_the_migration() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  drop(a);
  downgrade_head(&repo, &[(FOO, &["claude-code"])]);
  edit_skill(&b, FOO, "foo", "from b");
  let before = sync_files(&b);
  let commits = repo.commit_count();

  assert_attention(
    &default_cycle(&b, &repo),
    AttentionReason::MigrationRequired,
  );
  assert_eq!(sync_files(&b), before);
  assert_eq!(repo.commit_count(), commits);
}

#[test]
fn a_diverged_backup_waits_and_writes_nothing() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let b = conflict(tmp.path(), &repo);
  let before = sync_files(&b);
  let commits = repo.commit_count();
  let outcome = cycle(
    &b,
    &repo,
    AutoSyncOptions {
      restore: false,
      ..options(SyncPolicy::default())
    },
  );
  assert_attention(&outcome, AttentionReason::Diverged);
  assert_eq!(sync_files(&b), before);
  assert_eq!(repo.commit_count(), commits);
}

#[test]
fn restore_off_leaves_repository_changes() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  edit_skill(&a, FOO, "foo", "v2");
  backup_with(&a, &repo).unwrap();

  let outcome = cycle(
    &b,
    &repo,
    AutoSyncOptions {
      restore: false,
      ..options(SyncPolicy::default())
    },
  );
  assert_eq!(outcome.result, AutoSyncResult::UpToDate, "{:?}", outcome);
  assert!(skill_body(&b, FOO, "foo").contains("v1"));
}

#[test]
fn backup_off_leaves_local_changes() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (_a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  edit_skill(&b, FOO, "foo", "from b");
  let commits = repo.commit_count();

  let outcome = cycle(
    &b,
    &repo,
    AutoSyncOptions {
      backup: false,
      ..options(SyncPolicy::default())
    },
  );
  assert_eq!(outcome.result, AutoSyncResult::UpToDate, "{:?}", outcome);
  assert_eq!(repo.commit_count(), commits);
  assert_eq!(status(&b).unwrap().changes.len(), 1);

  let outcome = default_cycle(&b, &repo);
  assert_eq!(outcome.result, AutoSyncResult::Synced, "{:?}", outcome);
  assert!(outcome.backed_up);
  assert!(head_skill(&repo, FOO, "foo").contains("from b"));
}

#[test]
fn a_busy_lock_skips_the_cycle() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let (a, b) = synced_pair(tmp.path(), &repo, |a| add_skill(a, FOO, "foo", "v1"));
  edit_skill(&a, FOO, "foo", "v2");
  backup_with(&a, &repo).unwrap();

  let guard = ops_guard(&b).unwrap();
  let outcome = default_cycle(&b, &repo);
  drop(guard);
  assert_eq!(outcome.result, AutoSyncResult::Skipped);
  assert!(skill_body(&b, FOO, "foo").contains("v1"));
}

#[test]
fn a_disconnected_library_does_not_sync() {
  let tmp = tempfile::tempdir().unwrap();
  let repo = FakeRepo::initialized();
  let b = computer(tmp.path(), "b", &["claude-code"]);
  disconnect(&b).unwrap();
  assert_eq!(
    default_cycle(&b, &repo).result,
    AutoSyncResult::NotConnected
  );
}
