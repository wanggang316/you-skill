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
    let mut cmd = Command::cargo_bin("youskill").expect("youskill binary");
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
