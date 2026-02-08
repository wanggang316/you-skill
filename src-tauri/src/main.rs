#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod models;
mod paths;
mod tray;

use commands::{
  agents::list_agents,
  backup::{backup_skills, get_backup_folder, set_backup_folder, open_backup_folder, get_last_backup_time},
  install::{install_skill, detect_github_skills, detect_zip_skills, install_zip_skill, install_github_skill},
  manage::{copy_skill, delete_skill, move_skill},
  manage::{check_canonical_skill, unify_skill, set_agent_link},
  remote::{fetch_remote_skills, record_skill_install},
  scan::{add_scan_root, get_scan_roots, remove_scan_root, scan_local_skills},
  settings::{get_settings, update_settings},
  skill::read_skill_readme,
};
use tray::{setup_tray, update_tray_skills, TrayState};

#[tauri::command]
fn ping() -> String {
  "pong".to_string()
}

fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    .manage(TrayState {
      skills: std::sync::Mutex::new(Vec::new()),
    })
    .setup(|app| {
      // Setup tray icon and menu
      if let Err(e) = setup_tray(app.handle()) {
        eprintln!("Failed to setup tray: {}", e);
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      ping,
      scan_local_skills,
      get_scan_roots,
      add_scan_root,
      remove_scan_root,
      delete_skill,
      move_skill,
      copy_skill,
      check_canonical_skill,
      unify_skill,
      set_agent_link,
      get_settings,
      update_settings,
      fetch_remote_skills,
      record_skill_install,
      install_skill,
      list_agents,
      detect_github_skills,
      detect_zip_skills,
      install_zip_skill,
      install_github_skill,
      get_backup_folder,
      set_backup_folder,
      open_backup_folder,
      backup_skills,
      get_last_backup_time,
      read_skill_readme,
      update_tray_skills
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
