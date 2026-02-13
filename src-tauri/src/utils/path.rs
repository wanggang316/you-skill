use std::path::{Path, PathBuf};

pub fn is_under_canonical(path: &Path, global_canonical: &Path) -> bool {
  let path_str = path.to_string_lossy();
  let global_str = global_canonical.to_string_lossy();
  path_str.starts_with(global_str.as_ref())
}

pub fn expand_home(path: &str) -> PathBuf {
  if path.starts_with("~/") {
    if let Some(home) = dirs_next::home_dir() {
      return home.join(path.trim_start_matches("~/"));
    }
  }
  PathBuf::from(path)
}
