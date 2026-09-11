use crate::models::{ScanRootSuggestion, ScanRootView};
use crate::services::env::Env;
use crate::services::scan_roots_service;

/// Registered scan paths, flagging the ones whose folder no longer exists.
#[tauri::command]
pub fn list_scan_roots() -> Result<Vec<ScanRootView>, String> {
  scan_roots_service::list_scan_roots()
}

#[tauri::command]
pub fn add_scan_root(name: String, path: String) -> Result<ScanRootView, String> {
  let env = Env::current()?;
  scan_roots_service::add_scan_root(&env, name, path)
}

#[tauri::command]
pub fn remove_scan_root(path: String) -> Result<(), String> {
  scan_roots_service::remove_scan_root(&path)
}

/// Agent skills directories worth registering, for an empty list.
#[tauri::command]
pub fn suggest_scan_roots() -> Result<Vec<ScanRootSuggestion>, String> {
  let env = Env::current()?;
  scan_roots_service::suggest_scan_roots(&env)
}
