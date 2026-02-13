use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FolderHelper;

impl FolderHelper {
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
