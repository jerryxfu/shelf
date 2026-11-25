//! The tray icon (the menu bar icon on macOS) and its menu.

use tauri::{
    AppHandle, Manager, Rect, Wry,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

use crate::panel;

pub(crate) const TRAY: &str = "main";
const TOGGLE: &str = "toggle";
const QUIT: &str = "quit";

/// The "Show Shelf" / "Hide Shelf" item, kept so that its text can follow the panel.
struct ToggleItem(MenuItem<Wry>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, TOGGLE, toggle_text(false), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit Shelf", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &PredefinedMenuItem::separator(app)?, &quit])?;

    let tray = TrayIconBuilder::with_id(TRAY)
        .tooltip("Shelf")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            TOGGLE => panel::toggle(app),
            QUIT => app.exit(0),
            _ => {}
        });

    // macOS draws a template image in the menu bar's own colour, light or dark. Elsewhere the tray shows the app's icon.
    #[cfg(target_os = "macos")]
    let tray = tray
        .icon(tauri::include_image!("icons/tray-template.png"))
        .icon_as_template(true);
    #[cfg(not(target_os = "macos"))]
    let tray = match app.default_window_icon() {
        Some(icon) => tray.icon(icon.clone()),
        None => tray,
    };

    tray.build(app)?;
    app.manage(ToggleItem(toggle));
    Ok(())
}

/// Makes the menu say what a click will do now.
pub fn sync(app: &AppHandle) {
    if let Some(item) = app.try_state::<ToggleItem>() {
        let _ = item.0.set_text(toggle_text(panel::is_open(app)));
    }
}

/// Where the icon is on screen, for the panel to open next to it. `None` where the system does not say (Linux).
pub fn rect(app: &AppHandle) -> Option<Rect> {
    app.tray_by_id(TRAY)?.rect().ok()?
}

/// What the menu's first item says now, for the test aid to read.
#[cfg(feature = "dev-control")]
pub fn toggle_label(app: &AppHandle) -> Option<String> {
    app.try_state::<ToggleItem>()?.0.text().ok()
}

fn toggle_text(open: bool) -> &'static str {
    if open { "Hide Shelf" } else { "Show Shelf" }
}
