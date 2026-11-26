//! What the interface can ask the core itself to do. A module's own commands are in its folder.

use tauri::AppHandle;

use crate::panel;

#[tauri::command]
pub fn panel_hide(app: AppHandle) -> tauri::Result<()> {
    // Later, on the main thread: hiding destroys the web view this call came from.
    let handle = app.clone();
    app.run_on_main_thread(move || panel::hide(&handle))
}
