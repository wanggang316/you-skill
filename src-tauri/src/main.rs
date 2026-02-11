#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod agent_apps;
mod commands;
mod config;
mod models;
mod paths;
mod tray;

use commands::{
  agent_apps::{
    add_agent_app, list_all_agent_apps, list_internal_agent_apps, list_local_agent_apps,
    list_user_agent_apps, remove_agent_app, validate_agent_app,
  },
  agents::list_agents,
  backup::{
    backup_skills, get_backup_folder, get_last_backup_time, open_backup_folder, set_backup_folder,
  },
  install::{
    detect_folder_skills, detect_github_skills, detect_zip_skills, install_folder_skill,
    install_github_skill, install_skill, install_zip_skill,
  },
  manage::{check_canonical_skill, set_agent_link, unify_skill},
  manage::{copy_skill, delete_skill, delete_skill_complete, move_skill},
  remote::{fetch_remote_skills, fetch_skills_by_names, record_skill_install},
  scan::{add_scan_root, get_scan_roots, remove_scan_root, scan_local_skills},
  settings::{get_settings, update_settings},
  skill::{open_in_file_manager, read_skill_readme},
  skill_lock::{
    add_skill_to_lock, get_all_locked_skills, get_skill_from_lock, read_skill_lock,
    remove_skill_from_lock, write_skill_lock,
  },
};
use tray::{setup_tray, update_tray_skills, TrayState};

#[tauri::command]
fn ping() -> String {
  "pong".to_string()
}

fn main() {
  // Set default log level: info for dev, warn for production
  #[cfg(debug_assertions)]
  let default_level = "info";
  #[cfg(not(debug_assertions))]
  let default_level = "warn";

  // Initialize tracing subscriber for logging
  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level)),
    )
    .init();

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

      // Handle window close event - hide to tray instead of quitting
      let main_window = app.get_webview_window("main").unwrap();
      let window_clone = main_window.clone();

      main_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
          // Prevent the window from closing
          api.prevent_close();
          // Just hide the window instead
          let _ = window_clone.hide();
        }
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      ping,
      scan_local_skills,
      get_scan_roots,
      add_scan_root,
      remove_scan_root,
      delete_skill,
      delete_skill_complete,
      move_skill,
      copy_skill,
      check_canonical_skill,
      unify_skill,
      set_agent_link,
      get_settings,
      update_settings,
      fetch_remote_skills,
      fetch_skills_by_names,
      record_skill_install,
      install_skill,
      list_agents,
      list_local_agent_apps,
      list_all_agent_apps,
      list_internal_agent_apps,
      list_user_agent_apps,
      add_agent_app,
      remove_agent_app,
      validate_agent_app,
      detect_github_skills,
      detect_zip_skills,
      install_zip_skill,
      install_github_skill,
      detect_folder_skills,
      install_folder_skill,
      get_backup_folder,
      set_backup_folder,
      open_backup_folder,
      backup_skills,
      get_last_backup_time,
      update_tray_skills,
      read_skill_readme,
      open_in_file_manager,
      read_skill_lock,
      write_skill_lock,
      add_skill_to_lock,
      remove_skill_from_lock,
      get_skill_from_lock,
      get_all_locked_skills
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
