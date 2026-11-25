//! What the interface can ask the core to do. A thin layer: the logic is in `shelf.rs` and `panel.rs`.

use std::{path::PathBuf, sync::Mutex};

use tauri::{AppHandle, State};

use crate::{
    panel,
    shelf::{Shelf, ShelfItem},
};

pub type ShelfState = Mutex<Shelf>;

/// What is on the shelf now. Files that have gone since the last look are dropped first.
#[tauri::command]
pub fn shelf_items(shelf: State<'_, ShelfState>) -> Vec<ShelfItem> {
    let mut shelf = shelf.lock().unwrap();
    shelf.refresh();
    shelf.items().to_vec()
}

#[tauri::command]
pub fn shelf_add(shelf: State<'_, ShelfState>, paths: Vec<PathBuf>) -> Vec<ShelfItem> {
    let mut shelf = shelf.lock().unwrap();
    shelf.add(paths);
    shelf.items().to_vec()
}

/// Takes an item off the shelf. The file stays where it is.
#[tauri::command]
pub fn shelf_remove(shelf: State<'_, ShelfState>, id: u64) -> Vec<ShelfItem> {
    let mut shelf = shelf.lock().unwrap();
    shelf.remove(id);
    shelf.items().to_vec()
}

#[tauri::command]
pub fn panel_hide(app: AppHandle) -> tauri::Result<()> {
    // Later, on the main thread: hiding destroys the web view this call came from.
    let handle = app.clone();
    app.run_on_main_thread(move || panel::hide(&handle))
}
