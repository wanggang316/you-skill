use std::fs;
use std::path::{Component, Path, PathBuf};

pub const YOUSKILL_DIR: &str = ".youskill";
pub const HUB_DIR: &str = "skills";
pub const LOCK_FILE: &str = ".skill-lock.json";
pub const TRASH_DIR: &str = ".trash";
const MAX_SKILL_NAME_LEN: usize = 64;

pub fn youskill_root(home: &Path) -> PathBuf {
  home.join(YOUSKILL_DIR)
}

/// The pre-hub canonical directory (`~/.agents/skills`), still read during migration.
pub fn legacy_agents_root(home: &Path) -> PathBuf {
  home.join(".agents").join("skills")
}

pub fn expand_home(path: &str) -> PathBuf {
  match dirs_next::home_dir() {
    Some(home) => expand_home_with(path, &home),
    None => PathBuf::from(path),
  }
}

pub fn expand_home_with(path: &str, home: &Path) -> PathBuf {
  if path == "~" {
    return home.to_path_buf();
  }
  if let Some(rest) = path.strip_prefix("~/") {
    return home.join(rest);
  }
  if let Some(rest) = path.strip_prefix("~\\") {
    return home.join(rest);
  }
  PathBuf::from(path)
}

pub fn path_to_string(path: &Path) -> String {
  path.to_string_lossy().to_string()
}

/// A skill name is used verbatim as a directory name inside the hub and every install target.
pub fn validate_skill_name(name: &str) -> Result<(), String> {
  if name.is_empty() || name != name.trim() {
    return Err("Skill name must not be empty or padded with whitespace".to_string());
  }
  if name == "." || name == ".." {
    return Err(format!("Invalid skill name: '{}'", name));
  }
  if name.starts_with('.') {
    return Err(format!("Skill name must not start with '.': '{}'", name));
  }
  if name.chars().count() > MAX_SKILL_NAME_LEN {
    return Err(format!(
      "Skill name is longer than {} characters: '{}'",
      MAX_SKILL_NAME_LEN, name
    ));
  }
  if name.ends_with('.') {
    return Err(format!("Skill name must not end with '.': '{}'", name));
  }
  const RESERVED: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
  if name
    .chars()
    .any(|ch| ch.is_control() || RESERVED.contains(&ch))
  {
    return Err(format!(
      "Skill name contains characters that are not allowed: '{}'",
      name
    ));
  }
  Ok(())
}

/// Normalize a user-supplied directory path: expand `~`, drop `.` and trailing separators,
/// require an absolute path to an existing directory. The result keeps the user's spelling
/// (no canonicalization) so it can be stored and shown back.
pub fn normalize_dir_path(input: &str, home: &Path) -> Result<PathBuf, String> {
  let trimmed = input.trim();
  if trimmed.is_empty() {
    return Err("Path is required".to_string());
  }
  let expanded = expand_home_with(trimmed, home);
  if !expanded.is_absolute() {
    return Err(format!("Path must be absolute: {}", trimmed));
  }
  let normalized: PathBuf = expanded
    .components()
    .filter(|component| !matches!(component, Component::CurDir))
    .collect();
  if !normalized.is_dir() {
    return Err(format!(
      "Directory does not exist: {}",
      normalized.to_string_lossy()
    ));
  }
  Ok(normalized)
}

fn lexical_normalize(path: &Path) -> PathBuf {
  let mut out = PathBuf::new();
  for component in path.components() {
    match component {
      Component::CurDir => {},
      Component::ParentDir => {
        out.pop();
      },
      other => out.push(other.as_os_str()),
    }
  }
  out
}

/// Resolve a path for comparison purposes: canonicalize when it exists, otherwise
/// canonicalize the nearest existing ancestor and re-append the remaining components.
pub fn resolve_for_compare(path: &Path) -> PathBuf {
  if let Ok(canonical) = path.canonicalize() {
    return canonical;
  }
  let normalized = lexical_normalize(path);
  let mut remaining: Vec<PathBuf> = Vec::new();
  let mut cursor = normalized.clone();
  loop {
    if let Ok(canonical) = cursor.canonicalize() {
      let mut resolved = canonical;
      for part in remaining.iter().rev() {
        resolved.push(part);
      }
      return resolved;
    }
    match (cursor.file_name(), cursor.parent()) {
      (Some(name), Some(parent)) => {
        remaining.push(PathBuf::from(name));
        cursor = parent.to_path_buf();
      },
      _ => return normalized,
    }
  }
}

pub fn same_path(a: &Path, b: &Path) -> bool {
  if a == b {
    return true;
  }
  resolve_for_compare(a) == resolve_for_compare(b)
}

pub fn is_within(child: &Path, ancestor: &Path) -> bool {
  resolve_for_compare(child).starts_with(resolve_for_compare(ancestor))
}

/// True when `link` is a symlink whose target resolves to `target`.
pub fn symlink_points_to(link: &Path, target: &Path) -> bool {
  let Ok(meta) = fs::symlink_metadata(link) else {
    return false;
  };
  if !meta.file_type().is_symlink() {
    return false;
  }
  let Ok(raw) = fs::read_link(link) else {
    return false;
  };
  let resolved = if raw.is_absolute() {
    raw
  } else {
    link.parent().map(|p| p.join(&raw)).unwrap_or(raw)
  };
  same_path(&resolved, target)
}

pub fn is_symlink(path: &Path) -> bool {
  fs::symlink_metadata(path)
    .map(|meta| meta.file_type().is_symlink())
    .unwrap_or(false)
}

/// Remove a directory, file, or symlink without following the link.
pub fn remove_path_any(path: &Path) -> Result<(), String> {
  let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
  let file_type = meta.file_type();
  if file_type.is_symlink() {
    #[cfg(windows)]
    {
      use std::os::windows::fs::FileTypeExt;
      if file_type.is_symlink_dir() {
        return fs::remove_dir(path).map_err(|e| e.to_string());
      }
    }
    return fs::remove_file(path).map_err(|e| e.to_string());
  }
  if file_type.is_dir() {
    fs::remove_dir_all(path).map_err(|e| e.to_string())
  } else {
    fs::remove_file(path).map_err(|e| e.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn skill_name_validation_table() {
    for ok in ["foo", "foo-bar", "Foo_Bar.v2", "中文技能", "a"] {
      assert!(validate_skill_name(ok).is_ok(), "{ok} should be valid");
    }
    for bad in [
      "", " foo", "foo ", ".", "..", ".hidden", "a/b", "a\\b", "foo.", "a:b", "a*b", "a?b", "a\"b",
      "a<b", "a>b", "a|b",
    ] {
      assert!(
        validate_skill_name(bad).is_err(),
        "{bad:?} should be invalid"
      );
    }
    let long = "x".repeat(65);
    assert!(validate_skill_name(&long).is_err());
  }

  #[test]
  fn normalize_strips_trailing_separator_and_cur_dir() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let input = format!("{}/./", dir.path().to_string_lossy());
    let normalized = normalize_dir_path(&input, home).unwrap();
    assert_eq!(normalized, dir.path());
  }

  #[test]
  fn normalize_expands_home() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("proj")).unwrap();
    let normalized = normalize_dir_path("~/proj", dir.path()).unwrap();
    assert_eq!(normalized, dir.path().join("proj"));
  }

  #[test]
  fn normalize_rejects_relative_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(normalize_dir_path("relative/path", dir.path()).is_err());
    let missing = dir.path().join("missing");
    assert!(normalize_dir_path(&missing.to_string_lossy(), dir.path()).is_err());
  }

  #[test]
  fn is_within_handles_parent_components() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("a/b")).unwrap();
    assert!(is_within(&root.join("a/b/c"), root));
    assert!(!is_within(&root.join("a/../.."), root));
  }

  #[cfg(unix)]
  #[test]
  fn same_path_resolves_symlinked_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    fs::create_dir_all(&real).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(same_path(&link, &real));
    assert!(symlink_points_to(&link, &real));
    assert!(!symlink_points_to(&real, &link));
  }

  #[cfg(unix)]
  #[test]
  fn remove_path_any_removes_symlink_not_target() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    fs::create_dir_all(&real).unwrap();
    fs::write(real.join("f"), "x").unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    remove_path_any(&link).unwrap();
    assert!(!link.exists() && !is_symlink(&link));
    assert!(real.join("f").exists());
    remove_path_any(&real).unwrap();
    assert!(!real.exists());
  }
}
