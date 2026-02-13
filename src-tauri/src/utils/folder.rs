use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

  pub fn find_dirs_containing_file(root: &Path, file_name: &str) -> Result<Vec<PathBuf>, String> {
    if !root.exists() || !root.is_dir() {
      return Err(format!("Directory does not exist: {}", root.to_string_lossy()));
    }

    let mut dirs = Vec::new();
    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
      if entry.file_type().is_file() && entry.file_name() == file_name {
        let Some(parent) = entry.path().parent() else {
          continue;
        };
        dirs.push(parent.to_path_buf());
      }
    }

    Ok(dirs)
  }
}
