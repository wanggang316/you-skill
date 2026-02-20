use std::fs;
use std::path::{Path, PathBuf};

pub fn canonical_skills_root() -> Result<PathBuf, String> {
  let home = dirs_next::home_dir().ok_or("Could not find home directory")?;
  Ok(home.join(".agents").join("skills"))
}

pub fn expand_home(path: &str) -> PathBuf {
  if path.starts_with("~/") {
    if let Some(home) = dirs_next::home_dir() {
      return home.join(path.trim_start_matches("~/"));
    }
  }
  PathBuf::from(path)
}

pub fn remove_path_any(path: &Path) -> Result<(), String> {
  if path.is_dir() && !path.is_symlink() {
    fs::remove_dir_all(path).map_err(|e| e.to_string())
  } else {
    fs::remove_file(path).map_err(|e| e.to_string())
  }
}
