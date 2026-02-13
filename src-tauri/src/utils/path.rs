use std::path::PathBuf;

pub fn expand_home(path: &str) -> PathBuf {
  if path.starts_with("~/") {
    if let Some(home) = dirs_next::home_dir() {
      return home.join(path.trim_start_matches("~/"));
    }
  }
  PathBuf::from(path)
}
