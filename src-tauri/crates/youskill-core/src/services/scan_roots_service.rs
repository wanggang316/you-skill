//! Registered scan roots: folders the user keeps scanning for skills. Scanning itself stays
//! in `scan_service`; this module only remembers where to scan and what the last scan found.

use crate::models::{InstallScope, ScanRoot, ScanRootSuggestion, ScanRootView};
use crate::services::env::Env;
use crate::utils::path::{normalize_dir_path, path_to_string, same_path};
use crate::utils::time::now_rfc3339;
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_scan_roots() -> Result<Vec<ScanRootView>, String> {
  Ok(
    load_roots()?
      .into_iter()
      .map(|root| ScanRootView {
        missing: !Path::new(&root.path).is_dir(),
        root,
      })
      .collect(),
  )
}

pub fn add_scan_root(env: &Env, name: String, path: String) -> Result<ScanRootView, String> {
  let dir = normalize_dir_path(&path, &env.home)?;
  let path = path_to_string(&dir);
  let name = match name.trim() {
    "" => dir
      .file_name()
      .map(|value| value.to_string_lossy().to_string())
      .unwrap_or_else(|| path.clone()),
    value => value.to_string(),
  };

  let mut roots = load_roots()?;
  let root = insert_root(&mut roots, name, path)?;
  save_roots(&roots)?;
  Ok(ScanRootView {
    missing: false,
    root,
  })
}

pub fn remove_scan_root(path: &str) -> Result<(), String> {
  let mut roots = load_roots()?;
  remove_root(&mut roots, path)?;
  save_roots(&roots)
}

/// Record the result of a scan. Paths that are not registered are ignored, so every scan
/// entry point can call this without checking first.
pub fn touch_scan_root(path: &str, found: usize) -> Result<(), String> {
  let mut roots = load_roots()?;
  if !touch_root(&mut roots, path, found, now_rfc3339()) {
    return Ok(());
  }
  save_roots(&roots)
}

/// User-level skills directories of the installed agent apps that exist but are not
/// registered yet. Several agents can share one directory, so they are merged per path.
pub fn suggest_scan_roots(env: &Env) -> Result<Vec<ScanRootSuggestion>, String> {
  let roots = load_roots()?;
  let mut out: Vec<ScanRootSuggestion> = Vec::new();

  for app in &env.agent_apps {
    let Ok(dir) = env.agent_root(app, InstallScope::User, None) else {
      continue;
    };
    if !dir.is_dir() || roots.iter().any(|root| is_same(&root.path, &dir)) {
      continue;
    }
    match out
      .iter_mut()
      .find(|suggestion| is_same(&suggestion.path, &dir))
    {
      Some(suggestion) => suggestion.agent_ids.push(app.id.clone()),
      None => out.push(ScanRootSuggestion {
        path: path_to_string(&dir),
        agent_ids: vec![app.id.clone()],
      }),
    }
  }

  Ok(out)
}

fn is_same(path: &str, dir: &Path) -> bool {
  same_path(Path::new(path), dir)
}

fn insert_root(roots: &mut Vec<ScanRoot>, name: String, path: String) -> Result<ScanRoot, String> {
  if roots
    .iter()
    .any(|root| is_same(&root.path, Path::new(&path)))
  {
    return Err(format!("Scan path '{}' is already in the list", path));
  }
  let root = ScanRoot {
    name: unique_name(&name, roots),
    path,
    last_scanned_at: None,
    last_found: None,
  };
  roots.push(root.clone());
  Ok(root)
}

fn remove_root(roots: &mut Vec<ScanRoot>, path: &str) -> Result<(), String> {
  let before = roots.len();
  roots.retain(|root| !is_same(&root.path, Path::new(path)));
  if roots.len() == before {
    return Err(format!("Scan path '{}' not found", path));
  }
  Ok(())
}

/// Returns whether a registered root matched and was updated.
fn touch_root(roots: &mut [ScanRoot], path: &str, found: usize, timestamp: String) -> bool {
  let Some(root) = roots
    .iter_mut()
    .find(|root| is_same(&root.path, Path::new(path)))
  else {
    return false;
  };
  root.last_scanned_at = Some(timestamp);
  root.last_found = Some(found);
  true
}

fn unique_name(name: &str, existing: &[ScanRoot]) -> String {
  let base = if name.is_empty() { "folder" } else { name };
  let taken = |candidate: &str| {
    existing
      .iter()
      .any(|root| root.name.eq_ignore_ascii_case(candidate))
  };
  if !taken(base) {
    return base.to_string();
  }
  let mut index = 2;
  loop {
    let candidate = format!("{} {}", base, index);
    if !taken(&candidate) {
      return candidate;
    }
    index += 1;
  }
}

fn scan_roots_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("Unable to get config directory")?;
  Ok(config_dir.join("youskill").join("scan_roots.json"))
}

fn load_roots() -> Result<Vec<ScanRoot>, String> {
  let path = scan_roots_path()?;
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_roots(roots: &[ScanRoot]) -> Result<(), String> {
  let path = scan_roots_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(roots).map_err(|e| e.to_string())?;
  fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn roots() -> Vec<ScanRoot> {
    Vec::new()
  }

  #[test]
  fn insert_rejects_duplicate_paths_and_makes_names_unique() {
    let mut list = roots();
    insert_root(&mut list, "skills".to_string(), "/tmp/a".to_string()).unwrap();
    let second = insert_root(&mut list, "skills".to_string(), "/tmp/b".to_string()).unwrap();
    assert_eq!(second.name, "skills 2");

    let err = insert_root(&mut list, "again".to_string(), "/tmp/a/".to_string()).unwrap_err();
    assert!(err.contains("already in the list"));
    assert_eq!(list.len(), 2);
  }

  #[test]
  fn touch_records_time_and_count_only_for_known_paths() {
    let mut list = roots();
    insert_root(&mut list, "skills".to_string(), "/tmp/a".to_string()).unwrap();

    assert!(touch_root(&mut list, "/tmp/a", 3, "now".to_string()));
    assert_eq!(list[0].last_scanned_at.as_deref(), Some("now"));
    assert_eq!(list[0].last_found, Some(3));

    assert!(!touch_root(&mut list, "/tmp/other", 1, "later".to_string()));
    assert_eq!(list[0].last_scanned_at.as_deref(), Some("now"));
  }

  #[test]
  fn remove_reports_unknown_paths() {
    let mut list = roots();
    insert_root(&mut list, "skills".to_string(), "/tmp/a".to_string()).unwrap();

    assert!(remove_root(&mut list, "/tmp/other").is_err());
    remove_root(&mut list, "/tmp/a").unwrap();
    assert!(list.is_empty());
  }
}
