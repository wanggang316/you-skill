use tauri::{
  menu::{Menu, MenuItem, PredefinedMenuItem},
  AppHandle, Emitter, Manager,
};

const CHECK_FOR_UPDATES_ID: &str = "check-for-updates";
const OPEN_SETTINGS_ID: &str = "open-settings";

/// Uses the default app menu and puts "Check for Updates…" below "About YouSkill",
/// where Sparkle-based macOS apps show it, followed by "Settings…" in its own group.
pub fn setup_app_menu(app: &AppHandle) -> tauri::Result<()> {
  let menu = Menu::default(app)?;
  if let Some(app_submenu) = menu.items()?.first().and_then(|item| item.as_submenu()) {
    let check_item = MenuItem::with_id(
      app,
      CHECK_FOR_UPDATES_ID,
      "Check for Updates…",
      true,
      None::<&str>,
    )?;
    // No accelerator: the web view handles the user-configurable Settings shortcut,
    // and a native one would fire a second time and ignore a custom binding.
    let settings_item = MenuItem::with_id(app, OPEN_SETTINGS_ID, "Settings…", true, None::<&str>)?;
    // The default menu already has a separator after "About", so it ends up below "Settings…".
    app_submenu.insert_items(
      &[
        &check_item,
        &PredefinedMenuItem::separator(app)?,
        &settings_item,
      ],
      1,
    )?;
  }
  app.set_menu(menu)?;

  app.on_menu_event(|app, event| {
    let id = event.id().as_ref();
    if id == CHECK_FOR_UPDATES_ID || id == OPEN_SETTINGS_ID {
      show_main_window_and_emit(app, id);
    }
  });
  Ok(())
}

fn show_main_window_and_emit(app: &AppHandle, event: &str) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit(event, ());
  }
}
