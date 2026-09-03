//! One-time, idempotent migration from the pre-hub layout:
//! `~/.agents/skills` (canonical copies), `~/.agents/.skill-lock.json` (GitHub lock),
//! `<config_dir>/youskill/native-skill-lock.json`, `<project>/skills-lock.json`, plus every
//! agent's own skills directory. Nothing legacy is deleted.

use crate::models::{InstallMode, InstallScope, MigrationReport, SkillRecord, SkillSource};
use crate::services::env::Env;
use crate::services::install_service::{register_existing_install, write_target};
use crate::services::lock_service::{ops_guard, store};
use crate::utils::folder::{copy_dir, read_skill_name, CopyOpts, SKILL_MD};
use crate::utils::hash::{hash_dir, invalidate_hash_cache};
use crate::utils::path::{
  is_symlink, is_within, legacy_agents_root, same_path, symlink_points_to, validate_skill_name,
};
use crate::utils::time::now_rfc3339;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static CHECKED_THIS_PROCESS: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// Legacy lock formats (read-only)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyGithubEntry {
  #[serde(default)]
  pub source: String,
  #[serde(default)]
  pub source_url: String,
  #[serde(default)]
  pub skill_path: Option<String>,
  #[serde(default)]
  pub skill_folder_hash: Option<String>,
  #[serde(default)]
  pub installed_at: Option<String>,
}

impl LegacyGithubEntry {
  pub fn to_source(&self) -> SkillSource {
    let repo = if self.source.trim().is_empty() {
      crate::utils::github::GithubHelper::parse_github_url(&self.source_url)
        .map(|(owner, repo)| format!("{}/{}", owner, repo))
        .unwrap_or_default()
    } else {
      self.source.trim().to_string()
    };
    if repo.is_empty() {
      return SkillSource::None;
    }
    let url = if self.source_url.trim().is_empty() {
      format!("https://github.com/{}.git", repo)
    } else {
      self.source_url.trim().to_string()
    };
    SkillSource::Github {
      repo,
      url,
      skill_path: self.skill_path.clone().unwrap_or_default(),
      branch: Some("main".to_string()),
      remote_sha: self.skill_folder_hash.clone().filter(|s| !s.is_empty()),
      marketplace_id: None,
      latest_remote_sha: None,
      checked_at: None,
    }
  }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LegacyGithubLock {
  #[serde(default)]
  pub skills: HashMap<String, LegacyGithubEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyNativeEntry {
  #[serde(default)]
  pub installed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LegacyNativeLock {
  #[serde(default)]
  pub skills: HashMap<String, LegacyNativeEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyProjectEntry {
  #[serde(default)]
  pub source: String,
  #[serde(default)]
  pub source_type: String,
}

impl LegacyProjectEntry {
  pub fn to_source(&self) -> Option<SkillSource> {
    if self.source_type != "github" || self.source.trim().is_empty() {
      return None;
    }
    let repo = self.source.trim().to_string();
    Some(SkillSource::Github {
      url: format!("https://github.com/{}.git", repo),
      repo,
      skill_path: String::new(),
      branch: Some("main".to_string()),
      remote_sha: None,
      marketplace_id: None,
      latest_remote_sha: None,
      checked_at: None,
    })
  }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LegacyProjectLock {
  #[serde(default)]
  pub skills: HashMap<String, LegacyProjectEntry>,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
  let content = fs::read_to_string(path).ok()?;
  serde_json::from_str(&content).ok()
}

pub fn read_legacy_github_lock(path: &Path) -> Option<LegacyGithubLock> {
  read_json(path)
}

pub fn read_legacy_native_lock(path: &Path) -> Option<LegacyNativeLock> {
  read_json(path)
}

pub fn read_legacy_project_lock(path: &Path) -> Option<LegacyProjectLock> {
  read_json(path)
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

pub fn needs_migration(env: &Env) -> Result<bool, String> {
  let store = store(env);
  if !store.exists() {
    return Ok(true);
  }
  let lock = store.read()?;
  Ok(match lock.migration {
    Some(report) => report.completed_at.is_none() || !report.errors.is_empty(),
    None => true,
  })
}

/// Run the migration at most once per process when it is still pending.
pub fn ensure_migrated(env: &Env) -> Result<Option<MigrationReport>, String> {
  if CHECKED_THIS_PROCESS.swap(true, Ordering::SeqCst) {
    return Ok(None);
  }
  if !needs_migration(env)? {
    return Ok(None);
  }
  migrate_legacy(env).map(Some)
}

pub fn migrate_legacy(env: &Env) -> Result<MigrationReport, String> {
  let _ops = ops_guard();
  let mut report = MigrationReport::default();
  let legacy_root = legacy_agents_root(&env.home);
  let github_lock =
    read_legacy_github_lock(&env.home.join(".agents").join(".skill-lock.json")).unwrap_or_default();
  let native_lock =
    read_legacy_native_lock(&env.config_dir.join("native-skill-lock.json")).unwrap_or_default();

  fs::create_dir_all(&env.hub_root).map_err(|e| e.to_string())?;

  // 1. Legacy canonical copies become hub copies (preferred content).
  if legacy_root.is_dir() {
    migrate_root(
      env,
      &legacy_root,
      InstallScope::User,
      None,
      &mut report,
      |name, dir_name| {
        github_lock
          .skills
          .get(name)
          .or_else(|| github_lock.skills.get(dir_name))
          .map(|entry| (entry.to_source(), entry.installed_at.clone()))
          .or_else(|| {
            native_lock
              .skills
              .get(name)
              .map(|entry| (SkillSource::None, entry.installed_at.clone()))
          })
      },
    );
  }

  // 2. Every user-level agent root.
  for app in &env.agent_apps {
    let Ok(root) = env.agent_root(app, InstallScope::User, None) else {
      continue;
    };
    if same_path(&root, &legacy_root) || !root.is_dir() {
      continue;
    }
    migrate_root(env, &root, InstallScope::User, None, &mut report, |_, _| {
      None
    });
  }

  // 3. Registered projects: every agent's project root, with the project lock as hint.
  for project in &env.projects {
    let project_root = PathBuf::from(&project.path);
    if !project_root.is_dir() {
      continue;
    }
    let project_lock =
      read_legacy_project_lock(&project_root.join("skills-lock.json")).unwrap_or_default();
    let mut seen_roots: Vec<PathBuf> = Vec::new();
    for app in &env.agent_apps {
      let Ok(root) = env.agent_root(app, InstallScope::Project, Some(&project_root)) else {
        continue;
      };
      if !root.is_dir() || seen_roots.iter().any(|seen| same_path(seen, &root)) {
        continue;
      }
      seen_roots.push(root.clone());
      migrate_root(
        env,
        &root,
        InstallScope::Project,
        Some(project.path.clone()),
        &mut report,
        |name, dir_name| {
          project_lock
            .skills
            .get(name)
            .or_else(|| project_lock.skills.get(dir_name))
            .and_then(|entry| entry.to_source())
            .map(|source| (source, None))
        },
      );
    }
  }

  report.completed_at = Some(now_rfc3339());
  let saved = report.clone();
  store(env).update(|lock| {
    lock.migration = Some(saved);
    Ok(())
  })?;
  Ok(report)
}

/// Walk one skills directory: import unknown skills into the hub, then record the
/// directory (or relinked symlink) as an install target.
fn migrate_root<F>(
  env: &Env,
  root: &Path,
  scope: InstallScope,
  project_path: Option<String>,
  report: &mut MigrationReport,
  source_hint: F,
) where
  F: Fn(&str, &str) -> Option<(SkillSource, Option<String>)>,
{
  let Ok(entries) = fs::read_dir(root) else {
    return;
  };
  let legacy_root = legacy_agents_root(&env.home);
  let agent_ids: Vec<String> = env
    .agent_apps
    .iter()
    .filter(|app| {
      env
        .agent_root(app, scope, project_path.as_deref().map(Path::new))
        .map(|r| same_path(&r, root))
        .unwrap_or(false)
    })
    .map(|app| app.id.clone())
    .collect();

  let mut dirs: Vec<PathBuf> = entries
    .flatten()
    .map(|entry| entry.path())
    .filter(|path| path.is_dir() && path.join(SKILL_MD).is_file())
    .collect();
  dirs.sort();

  if is_within(root, &env.youskill_root) {
    return;
  }

  for dir in dirs {
    let dir_name = dir
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();
    let Some(name) = read_skill_name(&dir) else {
      continue;
    };
    if validate_skill_name(&name).is_err() {
      report.errors.push(format!(
        "{}: invalid skill name '{}'",
        dir.to_string_lossy(),
        name
      ));
      continue;
    }

    let hub_dir = env.hub_dir(&name);
    let linked_to_hub = symlink_points_to(&dir, &hub_dir);
    let linked_to_legacy = !linked_to_hub
      && is_symlink(&dir)
      && is_within(&dir.canonicalize().unwrap_or_default(), &legacy_root);

    // Import into the hub when unknown.
    let known = match store(env).get(&name) {
      Ok(record) => record.is_some(),
      Err(err) => {
        report.errors.push(err);
        continue;
      },
    };
    if !known {
      if linked_to_hub {
        continue;
      }
      if let Err(err) = import_into_hub(env, &name, &dir, &source_hint(&name, &dir_name)) {
        report
          .errors
          .push(format!("{}: {}", dir.to_string_lossy(), err));
        continue;
      }
      report.imported.push(name.clone());
    }

    // Relink symlinks that used to point at the legacy canonical dir.
    if linked_to_legacy {
      if let Err(err) = write_target(&hub_dir, &dir, InstallMode::Symlink) {
        report
          .errors
          .push(format!("{}: relink failed: {}", dir.to_string_lossy(), err));
        continue;
      }
    } else if is_symlink(&dir) && !linked_to_hub {
      // Linked somewhere we do not manage: leave it alone.
      continue;
    }

    let ids: Vec<String> = if agent_ids.is_empty() {
      vec![crate::services::agent_apps_service::SHARED_AGENTS_APP_ID.to_string()]
    } else {
      agent_ids.clone()
    };
    match register_existing_install(env, &name, &dir, scope, project_path.clone(), &ids) {
      Ok(()) => report.installs += 1,
      Err(err) => report
        .errors
        .push(format!("{}: {}", dir.to_string_lossy(), err)),
    }
  }
}

fn import_into_hub(
  env: &Env,
  name: &str,
  dir: &Path,
  hint: &Option<(SkillSource, Option<String>)>,
) -> Result<(), String> {
  let hub_dir = env.hub_dir(name);
  if hub_dir.exists() {
    // A previous partial run left the directory without a record: keep its content.
  } else {
    copy_dir(dir, &hub_dir, &CopyOpts::skill())?;
  }
  invalidate_hash_cache(&hub_dir);
  let hash = hash_dir(&hub_dir)?;
  let now = now_rfc3339();
  let (source, installed_at) = hint.clone().unwrap_or((SkillSource::None, None));
  let imported_at = installed_at
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| now.clone());
  store(env).update(|lock| {
    lock.skills.entry(name.to_string()).or_insert(SkillRecord {
      source,
      hash,
      imported_at,
      updated_at: now,
      installs: Vec::new(),
    });
    Ok(())
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{AgentApp, TargetState, UserProject};
  use crate::services::hub_service::get_hub_skill;

  fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  fn app(id: &str, global: &str, project: &str) -> AgentApp {
    AgentApp {
      id: id.to_string(),
      display_name: id.to_string(),
      project_path: Some(project.to_string()),
      global_path: Some(global.to_string()),
      is_user_custom: false,
    }
  }

  #[cfg(unix)]
  #[test]
  fn migrates_legacy_layout_idempotently() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    let env = Env::for_test(
      tmp.path(),
      vec![
        app("agents", "~/.agents/skills", ".agents/skills"),
        app("claude-code", "~/.claude/skills", ".claude/skills"),
        app("codex", "~/.codex/skills", ".agents/skills"),
      ],
      vec![UserProject {
        name: "proj".to_string(),
        path: project.to_string_lossy().to_string(),
      }],
    );
    let legacy = env.home.join(".agents/skills");
    write(&legacy, "foo/SKILL.md", "---\nname: foo\n---\nF");
    write(&legacy, "bar/SKILL.md", "---\nname: bar\n---\nB");
    write(
      &env.home.join(".agents"),
      ".skill-lock.json",
      r#"{"version":3,"skills":{"foo":{"source":"o/r","sourceType":"github","sourceUrl":"https://github.com/o/r.git","skillPath":"skills/foo/SKILL.md","skillFolderHash":"sha1","installedAt":"2026-01-01T00:00:00Z","updatedAt":"x"}}}"#,
    );
    write(
      &env.config_dir,
      "native-skill-lock.json",
      r#"{"version":1,"skills":{"bar":{"installedAt":"2026-02-02T00:00:00Z","updatedAt":"x"}}}"#,
    );
    // claude: symlink to legacy foo, real copy of bar (edited), plus an unmanaged skill.
    let claude = env.home.join(".claude/skills");
    fs::create_dir_all(&claude).unwrap();
    std::os::unix::fs::symlink(legacy.join("foo"), claude.join("foo")).unwrap();
    write(&claude, "bar/SKILL.md", "---\nname: bar\n---\nB-edited");
    write(&claude, "solo/SKILL.md", "---\nname: solo\n---\nS");
    // codex: symlink pointing somewhere else entirely.
    let codex = env.home.join(".codex/skills");
    fs::create_dir_all(&codex).unwrap();
    let elsewhere = tmp.path().join("elsewhere/foo");
    write(&elsewhere, "SKILL.md", "---\nname: foo\n---\nX");
    std::os::unix::fs::symlink(&elsewhere, codex.join("foo")).unwrap();
    // project: .agents/skills/baz with github project lock.
    write(
      &project,
      ".agents/skills/baz/SKILL.md",
      "---\nname: baz\n---\nZ",
    );
    write(
      &project,
      "skills-lock.json",
      r#"{"version":1,"skills":{"baz":{"source":"p/q","sourceType":"github","computedHash":"c"}}}"#,
    );

    let report = migrate_legacy(&env).unwrap();
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let mut imported = report.imported.clone();
    imported.sort();
    assert_eq!(imported, vec!["bar", "baz", "foo", "solo"]);

    let foo = get_hub_skill(&env, "foo").unwrap().unwrap();
    assert_eq!(foo.source.github_repo(), Some("o/r"));
    assert_eq!(foo.imported_at, "2026-01-01T00:00:00Z");
    assert_eq!(foo.hub_state, crate::models::HubState::Ok);
    // legacy dir recorded for `agents`, claude relinked to the hub.
    let by_path = |v: &crate::models::HubSkillView, suffix: &str| {
      v.installs
        .iter()
        .find(|i| i.record.path.ends_with(suffix))
        .cloned()
        .unwrap_or_else(|| panic!("no install ending with {suffix}"))
    };
    assert_eq!(
      by_path(&foo, ".agents/skills/foo").record.agent_ids,
      vec!["agents"]
    );
    let claude_foo = by_path(&foo, ".claude/skills/foo");
    assert_eq!(claude_foo.record.mode, InstallMode::Symlink);
    assert_eq!(claude_foo.state, TargetState::InSync);
    assert!(symlink_points_to(&claude.join("foo"), &env.hub_dir("foo")));
    // codex link to an unmanaged location is left alone and not recorded.
    assert!(foo
      .installs
      .iter()
      .all(|i| !i.record.path.contains(".codex")));
    assert!(symlink_points_to(&codex.join("foo"), &elsewhere));

    let bar = get_hub_skill(&env, "bar").unwrap().unwrap();
    assert_eq!(bar.source, SkillSource::None);
    assert_eq!(bar.imported_at, "2026-02-02T00:00:00Z");
    let claude_bar = by_path(&bar, ".claude/skills/bar");
    assert_eq!(claude_bar.record.mode, InstallMode::Copy);
    assert_eq!(claude_bar.state, TargetState::Modified);

    let solo = get_hub_skill(&env, "solo").unwrap().unwrap();
    assert_eq!(solo.installs.len(), 1);
    assert_eq!(solo.installs[0].state, TargetState::InSync);

    let baz = get_hub_skill(&env, "baz").unwrap().unwrap();
    assert_eq!(baz.source.github_repo(), Some("p/q"));
    let baz_install = &baz.installs[0];
    assert_eq!(baz_install.record.scope, InstallScope::Project);
    let mut ids = baz_install.record.agent_ids.clone();
    ids.sort();
    assert_eq!(ids, vec!["agents", "codex"]);

    // Legacy files untouched.
    assert!(legacy.join("foo/SKILL.md").is_file());
    assert!(env.home.join(".agents/.skill-lock.json").is_file());

    // Re-run: skill records are unchanged (only the migration report is rewritten).
    let before = crate::services::lock_service::read_lock_file(&env.lock_path).unwrap();
    let second = migrate_legacy(&env).unwrap();
    assert!(second.imported.is_empty());
    assert!(second.errors.is_empty());
    let after = crate::services::lock_service::read_lock_file(&env.lock_path).unwrap();
    assert_eq!(before.skills, after.skills);
    assert!(!needs_migration(&env).unwrap());
  }

  #[test]
  fn fresh_machine_migration_is_a_noop() {
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::for_test(tmp.path(), vec![], vec![]);
    assert!(needs_migration(&env).unwrap());
    let report = migrate_legacy(&env).unwrap();
    assert!(report.imported.is_empty());
    assert!(env.lock_path.is_file());
    assert!(!needs_migration(&env).unwrap());
  }
}
