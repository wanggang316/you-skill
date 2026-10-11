use tauri::{
  menu::{Menu, MenuItem},
  AppHandle, Emitter, Manager,
};

const CHECK_FOR_UPDATES_ID: &str = "check-for-updates";

/// Uses the default app menu and puts "Check for Updates…" below "About YouSkill",
/// where Sparkle-based macOS apps show it.
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
    app_submenu.insert(&check_item, 1)?;
  }
  app.set_menu(menu)?;

  app.on_menu_event(|app, event| {
    if event.id() == CHECK_FOR_UPDATES_ID {
      check_for_updates(app);
    }
  });
  Ok(())
}

fn check_for_updates(app: &AppHandle) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit(CHECK_FOR_UPDATES_ID, ());
  }
}
