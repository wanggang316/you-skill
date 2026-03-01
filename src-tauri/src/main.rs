#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod commands;
mod config;
mod models;
mod services;
mod tray;
mod utils;

use commands::{
  agent_apps::{
    add_user_agent_app, list_local_agent_apps, refresh_agent_apps, remove_user_agent_app,
    update_user_agent_app,
  },
  remote::{fetch_remote_skills, fetch_skills_by_names, record_skill_install},
  settings::{
    backup_skills, get_settings, open_backup_folder, set_backup_folder, update_settings,
  },
  skill::{
    check_skill_version, check_skills_updates, delete_skill, detect_folder, detect_github_auto,
    detect_github_manual, detect_zip, install_from_github, install_from_native,
    install_from_unknown, list_skill_directory, list_skills, manage_skill_agent_apps,
    open_in_file_manager, read_skill_file, read_skill_relative_file, read_skill_relative_file_bytes,
    translate_skill_markdown,
  },
  user_projects::{add_user_project, list_user_projects, remove_user_project, update_user_project},
};
use tray::setup_tray;

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
          // Prevent window from closing
          api.prevent_close();
          // Just hide window instead
          let _ = window_clone.hide();
        }
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      ping,
      list_skills,
      delete_skill,
      get_settings,
      update_settings,
      fetch_remote_skills,
      fetch_skills_by_names,
      record_skill_install,
      list_local_agent_apps,
      refresh_agent_apps,
      add_user_agent_app,
      remove_user_agent_app,
      update_user_agent_app,
      detect_zip,
      detect_folder,
      detect_github_manual,
      detect_github_auto,
      install_from_native,
      install_from_github,
      check_skill_version,
      install_from_unknown,
      manage_skill_agent_apps,
      set_backup_folder,
      open_backup_folder,
      backup_skills,
      read_skill_file,
      list_skill_directory,
      read_skill_relative_file,
      read_skill_relative_file_bytes,
      translate_skill_markdown,
      open_in_file_manager,
      check_skills_updates,
      list_user_projects,
      add_user_project,
      update_user_project,
      remove_user_project
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
