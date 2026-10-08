//! End-to-end tests of the `youskill` binary against a hub in a temporary home directory.
//! The home is isolated through `HOME` (and the Windows equivalents), which is how the
//! core resolves every path.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use youskill_core::models::{
  InstallMode, InstallRecord, InstallScope, LockFile, SkillRecord, SkillSource,
};
use youskill_core::services::lock_service::write_lock_file;
use youskill_core::utils::hash::hash_dir;

struct Home {
  dir: TempDir,
}

impl Home {
  fn new() -> Self {
    Self {
      dir: tempfile::tempdir().expect("temp home"),
    }
  }

  fn path(&self) -> &Path {
    self.dir.path()
  }

  fn hub(&self) -> PathBuf {
    self.path().join(".youskill").join("skills")
  }

  fn lock_path(&self) -> PathBuf {
    self.path().join(".youskill").join(".skill-lock.json")
  }

  fn cmd(&self) -> Command {
    Command::from_std(self.raw_cmd())
  }

  /// The binary as a plain `std::process::Command`, for tests that spawn it themselves.
  fn raw_cmd(&self) -> std::process::Command {
    let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("youskill"));
    cmd.env_clear();
    cmd.env("PATH", std::env::var_os("PATH").unwrap_or_default());
    cmd.env("HOME", self.path());
    cmd.env("USERPROFILE", self.path());
    cmd.env("XDG_CONFIG_HOME", self.path().join(".config"));
    cmd.env("APPDATA", self.path().join("AppData").join("Roaming"));
    cmd
  }

  /// A skill directory with a valid `SKILL.md`, anywhere.
  fn write_skill(&self, dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
      dir.join("SKILL.md"),
      format!(
        "---\nname: {}\ndescription: A test skill\n---\n\n{}\n",
        name, body
      ),
    )
    .unwrap();
  }

  /// Put a skill into the hub and record it in the lock with the hash it has right now.
  fn add_hub_skill(&self, name: &str, installs: Vec<InstallRecord>) {
    let dir = self.hub().join(name);
    self.write_skill(&dir, name, "# Body");
    let hash = hash_dir(&dir).unwrap();
    let mut lock = if self.lock_path().is_file() {
      serde_json::from_str::<LockFile>(&fs::read_to_string(self.lock_path()).unwrap()).unwrap()
    } else {
      LockFile::default()
    };
    lock.skills.insert(
      name.to_string(),
      SkillRecord {
        source: SkillSource::None,
        hash,
        imported_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        installs,
      },
    );
    write_lock_file(&self.lock_path(), &lock).unwrap();
  }
}

fn copy_install(home: &Home, name: &str, target: &Path) -> InstallRecord {
  let hub_dir = home.hub().join(name);
  fs::create_dir_all(target.parent().unwrap()).unwrap();
  fs::create_dir_all(target).unwrap();
  fs::copy(hub_dir.join("SKILL.md"), target.join("SKILL.md")).unwrap();
  InstallRecord {
    scope: InstallScope::User,
    project_path: None,
    path: target.to_string_lossy().to_string(),
    mode: InstallMode::Copy,
    hash: hash_dir(&hub_dir).unwrap(),
    installed_at: "2026-01-01T00:00:00Z".to_string(),
    agent_ids: vec!["claude-code".to_string()],
  }
}

#[test]
fn list_empty_hub() {
  let home = Home::new();
  home
    .cmd()
    .arg("list")
    .assert()
    .success()
    .stdout(predicate::str::contains("No skills in the hub."));
  home
    .cmd()
    .args(["list", "--json"])
    .assert()
    .success()
    .stdout(predicate::str::starts_with("[]"));
}

#[test]
fn list_and_show_a_hub_skill() {
  let home = Home::new();
  home.add_hub_skill("demo", Vec::new());

  home
    .cmd()
    .arg("list")
    .assert()
    .success()
    .stdout(predicate::str::contains("NAME"))
    .stdout(predicate::str::contains("demo"))
    .stdout(predicate::str::contains("ok"));

  let output = home
    .cmd()
    .args(["show", "demo", "--json"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let view: serde_json::Value = serde_json::from_slice(&output).unwrap();
  assert_eq!(view["name"], "demo");
  assert_eq!(view["hubState"], "ok");
  assert_eq!(view["description"], "A test skill");
  assert_eq!(view["installs"].as_array().unwrap().len(), 0);

  home
    .cmd()
    .args(["show", "demo"])
    .assert()
    .success()
    .stdout(predicate::str::contains("Not installed anywhere."));
}

#[test]
fn show_unknown_skill_fails() {
  let home = Home::new();
  home
    .cmd()
    .args(["show", "nope"])
    .assert()
    .code(1)
    .stderr(predicate::str::contains("Skill 'nope' is not in the hub"));
  home
    .cmd()
    .args(["show", "nope", "--json"])
    .assert()
    .code(1)
    .stderr(predicate::str::contains(
      r#"{"error":"Skill 'nope' is not in the hub"}"#,
    ));
}

#[test]
fn status_reports_drift_with_exit_code() {
  let home = Home::new();
  let target = home.path().join(".claude").join("skills").join("demo");
  home.add_hub_skill("demo", Vec::new());
  let install = copy_install(&home, "demo", &target);
  home.add_hub_skill("demo", vec![install]);

  home
    .cmd()
    .args(["status", "--exit-code"])
    .assert()
    .success()
    .stdout(predicate::str::contains("All skills are in sync."));

  // Edit the target: the copy is now `modified` relative to the hub.
  fs::write(target.join("SKILL.md"), "---\nname: demo\n---\nchanged\n").unwrap();
  home
    .cmd()
    .arg("status")
    .assert()
    .success()
    .stdout(predicate::str::contains("demo"))
    .stdout(predicate::str::contains("1 modified"));
  home.cmd().args(["status", "--exit-code"]).assert().code(3);
  home
    .cmd()
    .args(["list", "--state", "modified"])
    .assert()
    .success()
    .stdout(predicate::str::contains("demo"));
  home
    .cmd()
    .args(["list", "--state", "outdated"])
    .assert()
    .success()
    .stdout(predicate::str::contains(
      "No skills in the requested states.",
    ));
}

#[test]
fn diff_against_target() {
  let home = Home::new();
  let target = home.path().join("elsewhere").join("demo");
  home.add_hub_skill("demo", Vec::new());
  copy_install(&home, "demo", &target);

  home
    .cmd()
    .args(["diff", "demo", "--target", target.to_str().unwrap()])
    .assert()
    .success()
    .stdout(predicate::str::contains("are identical"));

  fs::write(target.join("extra.md"), "more\n").unwrap();
  fs::write(target.join("SKILL.md"), "---\nname: demo\n---\nchanged\n").unwrap();
  home
    .cmd()
    .args(["diff", "demo", "--target", target.to_str().unwrap()])
    .assert()
    .code(1)
    .stdout(predicate::str::contains("--- /dev/null\n+++ b/extra.md"))
    .stdout(predicate::str::contains("--- a/SKILL.md\n+++ b/SKILL.md"))
    .stdout(predicate::str::contains("+changed"));

  home
    .cmd()
    .args(["diff", "demo"])
    .assert()
    .code(2)
    .stderr(predicate::str::contains("--target"));
  home
    .cmd()
    .args(["diff", "demo", "--source"])
    .assert()
    .code(1)
    .stderr(predicate::str::contains("no source to compare with"));
}

#[test]
fn agents_and_projects_list() {
  let home = Home::new();
  let output = home
    .cmd()
    .args(["agents", "--all", "--json"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let apps: serde_json::Value = serde_json::from_slice(&output).unwrap();
  assert!(apps
    .as_array()
    .unwrap()
    .iter()
    .any(|app| app["id"] == "claude-code"));

  home
    .cmd()
    .args(["agents", "--all"])
    .assert()
    .success()
    .stdout(predicate::str::contains("claude-code"));

  home
    .cmd()
    .args(["projects", "list"])
    .assert()
    .success()
    .stdout(predicate::str::contains("No registered projects."));
  home
    .cmd()
    .args(["projects", "--json"])
    .assert()
    .success()
    .stdout(predicate::str::starts_with("[]"));
}

// ---------------------------------------------------------------------------
// Mutating commands
// ---------------------------------------------------------------------------

/// Make `claude-code` and `agents` detectable: their user-level roots' parents exist.
fn with_agents(home: &Home) {
  fs::create_dir_all(home.path().join(".claude")).unwrap();
  fs::create_dir_all(home.path().join(".agents")).unwrap();
}

fn show_json(home: &Home, name: &str) -> serde_json::Value {
  let output = home
    .cmd()
    .args(["show", name, "--json"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  serde_json::from_slice(&output).unwrap()
}

#[test]
fn install_and_uninstall_user_level() {
  let home = Home::new();
  with_agents(&home);
  home.add_hub_skill("demo", Vec::new());

  home
    .cmd()
    .args(["install", "demo", "-a", "claude-code", "--mode", "copy"])
    .assert()
    .success()
    .stdout(predicate::str::contains(
      "Installed 'demo' to user level (claude-code)",
    ));
  let target = home.path().join(".claude").join("skills").join("demo");
  assert!(target.join("SKILL.md").is_file());
  let view = show_json(&home, "demo");
  let installs = view["installs"].as_array().unwrap();
  assert_eq!(installs.len(), 1);
  assert_eq!(installs[0]["state"], "in_sync");
  assert_eq!(installs[0]["agentIds"][0], "claude-code");

  // Unknown agent ids are rejected before anything is written.
  home
    .cmd()
    .args(["install", "demo", "-a", "nope"])
    .assert()
    .code(1)
    .stderr(predicate::str::contains(
      "Unknown or undetected agent app 'nope'",
    ));

  // A locally edited copy is not deleted without --force.
  fs::write(target.join("SKILL.md"), "---\nname: demo\n---\nedited\n").unwrap();
  home
    .cmd()
    .args(["uninstall", "demo", "-a", "claude-code"])
    .assert()
    .code(2)
    .stderr(predicate::str::contains("has local changes"));
  assert!(target.is_dir());
  home
    .cmd()
    .args(["uninstall", "demo", "-a", "claude-code", "--force"])
    .assert()
    .success();
  assert!(!target.exists());
  assert!(show_json(&home, "demo")["installs"]
    .as_array()
    .unwrap()
    .is_empty());
}

#[test]
fn install_into_a_project_registers_it() {
  let home = Home::new();
  with_agents(&home);
  home.add_hub_skill("demo", Vec::new());
  let project = home.path().join("work").join("app");
  fs::create_dir_all(&project).unwrap();

  home
    .cmd()
    .args([
      "install",
      "demo",
      "-a",
      "claude-code,agents",
      "--mode",
      "symlink",
      "--project",
      project.to_str().unwrap(),
    ])
    .assert()
    .success();
  assert!(project.join(".claude/skills/demo").is_symlink());
  assert!(project.join(".agents/skills/demo").is_symlink());
  home
    .cmd()
    .args(["projects", "list"])
    .assert()
    .success()
    .stdout(predicate::str::contains(project.to_str().unwrap()));

  // `-p` without a value means the current directory.
  home
    .cmd()
    .current_dir(&project)
    .args(["uninstall", "demo", "-a", "all", "-p"])
    .assert()
    .success();
  assert!(!project.join(".claude/skills/demo").exists());
  assert!(show_json(&home, "demo")["installs"]
    .as_array()
    .unwrap()
    .is_empty());

  home
    .cmd()
    .args(["projects", "remove", project.to_str().unwrap()])
    .assert()
    .success();
  home
    .cmd()
    .args(["projects", "--json"])
    .assert()
    .success()
    .stdout(predicate::str::starts_with("[]"));
}

#[test]
fn concurrent_installs_keep_every_record() {
  let home = Home::new();
  with_agents(&home);
  home.add_hub_skill("demo", Vec::new());

  // Two processes write the same lock record at once; the ops lock serializes them.
  let mut first = home
    .raw_cmd()
    .args(["install", "demo", "-a", "claude-code"])
    .spawn()
    .unwrap();
  let mut second = home
    .raw_cmd()
    .args(["install", "demo", "-a", "agents"])
    .spawn()
    .unwrap();
  assert!(first.wait().unwrap().success());
  assert!(second.wait().unwrap().success());

  let view = show_json(&home, "demo");
  let mut ids: Vec<String> = view["installs"]
    .as_array()
    .unwrap()
    .iter()
    .flat_map(|install| install["agentIds"].as_array().unwrap().clone())
    .map(|id| id.as_str().unwrap().to_string())
    .collect();
  ids.sort();
  assert_eq!(ids, vec!["agents".to_string(), "claude-code".to_string()]);
}

#[test]
fn remove_needs_confirmation_or_yes() {
  let home = Home::new();
  with_agents(&home);
  home.add_hub_skill("demo", Vec::new());
  home
    .cmd()
    .args(["install", "demo", "-a", "claude-code"])
    .assert()
    .success();
  let target = home.path().join(".claude/skills/demo");

  home
    .cmd()
    .args(["remove", "demo"])
    .assert()
    .code(2)
    .stderr(predicate::str::contains("needs a terminal or --yes"));
  assert!(home.hub().join("demo").is_dir());

  home
    .cmd()
    .args(["remove", "demo", "-y"])
    .assert()
    .success()
    .stdout(predicate::str::contains("Removed 'demo'"));
  assert!(!home.hub().join("demo").exists());
  assert!(!target.exists());
  home.cmd().args(["show", "demo"]).assert().code(1);
}

#[test]
fn sync_push_and_adopt_targets() {
  let home = Home::new();
  with_agents(&home);
  home.add_hub_skill("demo", Vec::new());
  home
    .cmd()
    .args(["install", "demo", "-a", "claude-code", "--mode", "copy"])
    .assert()
    .success();
  let target = home.path().join(".claude/skills/demo");
  let hub = home.hub().join("demo");

  // Hub moves on: the target is outdated and push brings it up to date.
  fs::write(hub.join("extra.md"), "new\n").unwrap();
  home
    .cmd()
    .args(["sync", "demo", "accept-hub"])
    .assert()
    .success();
  assert_eq!(show_json(&home, "demo")["installs"][0]["state"], "outdated");
  home
    .cmd()
    .args(["sync", "demo", "push-targets"])
    .assert()
    .success()
    .stdout(predicate::str::contains("pushed to targets"));
  assert!(target.join("extra.md").is_file());
  assert_eq!(show_json(&home, "demo")["installs"][0]["state"], "in_sync");

  // Target edited: adopt takes it into the hub.
  fs::write(target.join("extra.md"), "edited\n").unwrap();
  home
    .cmd()
    .args(["sync", "demo", "adopt-target"])
    .assert()
    .code(1)
    .stderr(predicate::str::contains("exactly one --target"));
  home
    .cmd()
    .args([
      "sync",
      "demo",
      "adopt-target",
      "--target",
      target.to_str().unwrap(),
    ])
    .assert()
    .success();
  assert_eq!(
    fs::read_to_string(hub.join("extra.md")).unwrap(),
    "edited\n"
  );

  // Both sides changed: push is blocked until forced.
  fs::write(hub.join("extra.md"), "hub\n").unwrap();
  home
    .cmd()
    .args(["sync", "demo", "accept-hub"])
    .assert()
    .success();
  fs::write(target.join("extra.md"), "target\n").unwrap();
  home
    .cmd()
    .args(["sync", "demo", "push-targets"])
    .assert()
    .code(2)
    .stderr(predicate::str::contains("blocked"));
  home
    .cmd()
    .args(["sync", "demo", "push-targets", "--force", "--json"])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"applied\": true"));
  assert_eq!(
    fs::read_to_string(target.join("extra.md")).unwrap(),
    "hub\n"
  );
}

#[test]
fn import_from_folder_and_scan() {
  let home = Home::new();
  with_agents(&home);
  let source = home.path().join("src").join("skills");
  home.write_skill(&source.join("alpha"), "alpha", "A");
  home.write_skill(&source.join("beta"), "beta", "B");

  home
    .cmd()
    .args(["import", source.to_str().unwrap(), "--pick", "alpha"])
    .assert()
    .success()
    .stdout(predicate::str::contains("alpha"))
    .stdout(predicate::str::contains("imported"))
    .stdout(predicate::str::contains("beta").not());
  let view = show_json(&home, "alpha");
  assert_eq!(view["source"]["type"], "folder");
  assert_eq!(
    view["source"]["path"],
    source.join("alpha").to_str().unwrap()
  );

  home
    .cmd()
    .args(["import", source.to_str().unwrap(), "--pick", "alpha"])
    .assert()
    .code(1)
    .stderr(predicate::str::contains("already exists"));
  home
    .cmd()
    .args(["import", "--pick", "nope", source.to_str().unwrap()])
    .assert()
    .code(1)
    .stderr(predicate::str::contains("None of the picked names"));

  // A copy inside an agent directory scans as new and imports as that agent's install.
  let claude_copy = home.path().join(".claude/skills/beta");
  fs::create_dir_all(&claude_copy).unwrap();
  fs::copy(source.join("beta/SKILL.md"), claude_copy.join("SKILL.md")).unwrap();
  home
    .cmd()
    .args(["scan", home.path().to_str().unwrap()])
    .assert()
    .success()
    .stdout(predicate::str::contains("beta"))
    .stdout(predicate::str::contains("new"))
    .stdout(predicate::str::contains("--import"));
  home
    .cmd()
    .args([
      "scan",
      claude_copy.parent().unwrap().to_str().unwrap(),
      "--import",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Imported 1: beta"));
  let view = show_json(&home, "beta");
  assert_eq!(view["installs"][0]["agentIds"][0], "claude-code");
  assert_eq!(view["installs"][0]["state"], "in_sync");

  home
    .cmd()
    .args(["update", "--check"])
    .assert()
    .success()
    .stdout(predicate::str::contains("No skills with a GitHub source."));
}
