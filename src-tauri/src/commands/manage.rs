use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn delete_skill(path: String) -> Result<(), String> {
  let target = PathBuf::from(path);
  if target.is_dir() {
    fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
  } else if target.is_file() {
    fs::remove_file(&target).map_err(|e| e.to_string())?;
  }
  Ok(())
}

#[tauri::command]
pub fn move_skill(from: String, to: String) -> Result<(), String> {
  let from_path = PathBuf::from(from);
  let to_path = PathBuf::from(to);

  match fs::rename(&from_path, &to_path) {
    Ok(_) => Ok(()),
    Err(_) => {
      copy_dir_or_file(&from_path, &to_path)?;
      delete_skill(from_path.to_string_lossy().to_string())
    }
  }
}

#[tauri::command]
pub fn copy_skill(from: String, to: String) -> Result<(), String> {
  let from_path = PathBuf::from(from);
  let to_path = PathBuf::from(to);
  copy_dir_or_file(&from_path, &to_path)
}

fn copy_dir_or_file(from: &Path, to: &Path) -> Result<(), String> {
  if from.is_dir() {
    copy_dir_recursive(from, to)
  } else {
    if let Some(parent) = to.parent() {
      fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(from, to).map_err(|e| e.to_string())?;
    Ok(())
  }
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
  if !to.exists() {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
  }
  for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let entry_path = entry.path();
    let target_path = to.join(entry.file_name());
    if entry_path.is_dir() {
      copy_dir_recursive(&entry_path, &target_path)?;
    } else {
      fs::copy(&entry_path, &target_path).map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}
