use std::fs;
use std::path::{Path, PathBuf};

pub struct FolderHelper;

impl FolderHelper {
  pub fn exists(path: &Path) -> bool {
    path.exists()
  }

  pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
  }

  pub fn read_dir(path: &Path) -> Result<fs::ReadDir, String> {
    fs::read_dir(path).map_err(|e| e.to_string())
  }

  pub fn canonicalize(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|e| e.to_string())
  }

  pub fn symlink_metadata(path: &Path) -> Result<fs::Metadata, String> {
    fs::symlink_metadata(path).map_err(|e| e.to_string())
  }
}
