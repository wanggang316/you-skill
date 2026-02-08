use crate::models::LocalSkill;
use image::ImageDecoder;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

// Store skills in app state for tray menu
pub struct TrayState {
    pub skills: Mutex<Vec<LocalSkill>>,
}

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Build tray menu
    build_tray_menu(app)?;
    Ok(())
}

fn load_tray_icon() -> tauri::image::Image<'static> {
    // Load PNG from bytes and convert to RGBA
    let png_data = include_bytes!("../icons/icon-tray.png");
    let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(png_data)).unwrap();
    let (width, height) = decoder.dimensions();
    let mut rgba_vec = vec![0u8; (width * height * 4) as usize];

    {
        let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(png_data)).unwrap();
        ImageDecoder::read_image(decoder, &mut rgba_vec.as_mut_slice()).unwrap();
    }

    tauri::image::Image::new_owned(rgba_vec, width, height)
}

fn build_tray_menu(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Get skills from state or empty vec
    let skills: Vec<LocalSkill> = app
        .try_state::<TrayState>()
        .and_then(|state| state.skills.lock().ok().map(|s| s.clone()))
        .unwrap_or_default();

    // Build skills submenu
    let mut skills_submenu = SubmenuBuilder::new(app, "Skills List");
    if skills.is_empty() {
        let no_skills =
            MenuItem::with_id(app, "no-skills", "No skills found", false, None::<&str>)?;
        skills_submenu = skills_submenu.item(&no_skills);
    } else {
        for skill in &skills {
            let skill_name = skill.name.clone();
            let item = MenuItem::with_id(
                app,
                format!("skill:{}", skill.name),
                skill_name,
                true,
                None::<&str>,
            )?;
            skills_submenu = skills_submenu.item(&item);
        }
    }
    let skills_submenu = skills_submenu.build()?;

    // Create menu items
    let open_item = MenuItem::with_id(app, "open", "Open YouSkills", true, None::<&str>)?;
    let install_item = MenuItem::with_id(app, "install", "Install New", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Build menu
    let menu = Menu::with_items(
        app,
        &[
            &skills_submenu,
            &install_item,
            &open_item,
            &separator,
            &quit_item,
        ],
    )?;

    // Load tray icon
    let tray_icon = load_tray_icon();

    // Create tray icon (only created once at startup)
    let _tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "quit" => {
                    app.exit(0);
                }
                "open" => {
                    show_main_window(app);
                }
                "install" => {
                    show_main_window_with_install(app);
                }
                id if id.starts_with("skill:") => {
                    // Skill item clicked - just show the main window for now
                    show_main_window(app);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                let app = tray.app_handle();
                show_main_window(app);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_main_window_with_install(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();

        // Emit event to frontend to open install modal
        let _ = window.emit("open-install-modal", ());
    }
}

#[tauri::command]
pub async fn update_tray_skills(app: AppHandle, skills: Vec<LocalSkill>) -> Result<(), String> {
    // Update state only - don't recreate tray to prevent duplicates
    if let Some(state) = app.try_state::<TrayState>() {
        if let Ok(mut guard) = state.skills.lock() {
            *guard = skills;
        }
    }
    Ok(())
}
