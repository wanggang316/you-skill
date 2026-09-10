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
  hub::{
    check_source_updates, diff_skill, get_hub_skill, import_scanned, import_skills, install_skill,
    list_hub_skills, migrate_legacy, migration_status, remove_hub_skill, scan_folder, sync_skill,
    uninstall_skill,
  },
  instructions::{
    create_instruction, detect_instruction_files, detect_instruction_github, diff_instruction,
    get_instruction, import_instructions, install_instruction, list_instruction_files,
    list_instructions, read_instruction, read_instruction_file, remove_instruction,
    rename_instruction, sync_instruction, uninstall_instruction, write_instruction,
    write_instruction_file,
  },
  remote::{fetch_remote_skills, fetch_skills_by_names, record_skill_install},
  settings::{
    backup_skills, get_settings, list_openrouter_models, open_backup_folder, set_backup_folder,
    update_settings,
  },
  skill::{
    detect_folder, detect_github_auto, detect_github_manual, detect_zip, list_skill_directory,
    open_in_file_manager, read_skill_file, read_skill_relative_file,
    read_skill_relative_file_bytes, translate_skill_markdown,
  },
  user_projects::{
    add_user_project, add_workspace, list_memory_files, list_user_projects, list_workspaces,
    register_projects, remove_user_project, remove_workspace, scan_workspace, update_user_project,
  },
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
      list_hub_skills,
      get_hub_skill,
      import_skills,
      install_skill,
      uninstall_skill,
      remove_hub_skill,
      sync_skill,
      diff_skill,
      check_source_updates,
      scan_folder,
      import_scanned,
      migrate_legacy,
      migration_status,
      set_backup_folder,
      open_backup_folder,
      backup_skills,
      list_openrouter_models,
      read_skill_file,
      list_skill_directory,
      read_skill_relative_file,
      read_skill_relative_file_bytes,
      translate_skill_markdown,
      open_in_file_manager,
      list_user_projects,
      add_user_project,
      update_user_project,
      remove_user_project,
      list_workspaces,
      add_workspace,
      remove_workspace,
      scan_workspace,
      register_projects,
      list_memory_files,
      list_instructions,
      get_instruction,
      read_instruction,
      import_instructions,
      create_instruction,
      install_instruction,
      uninstall_instruction,
      remove_instruction,
      sync_instruction,
      diff_instruction,
      write_instruction,
      rename_instruction,
      list_instruction_files,
      read_instruction_file,
      write_instruction_file,
      detect_instruction_files,
      detect_instruction_github
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|_app_handle, _event| {
      // RunEvent::Reopen only exists on macOS, where clicking the Dock icon
      // fires it - show the main window again.
      #[cfg(target_os = "macos")]
      if let tauri::RunEvent::Reopen { .. } = _event {
        if let Some(window) = _app_handle.get_webview_window("main") {
          let _ = window.show();
          let _ = window.set_focus();
        }
      }
    });
}
