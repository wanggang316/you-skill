#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod models;
mod paths;

use commands::{
  agents::list_agents,
  install::install_skill,
  manage::{copy_skill, delete_skill, move_skill},
  remote::fetch_remote_skills,
  scan::{add_scan_root, get_scan_roots, remove_scan_root, scan_local_skills},
};

#[tauri::command]
fn ping() -> String {
  "pong".to_string()
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      ping,
      scan_local_skills,
      get_scan_roots,
      add_scan_root,
      remove_scan_root,
      delete_skill,
      move_skill,
      copy_skill,
      fetch_remote_skills,
      install_skill,
      list_agents
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
