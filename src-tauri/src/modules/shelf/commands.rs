//! What the interface can ask the shelf to do. A thin layer: the logic is in `items.rs`.
//!
//! Tauri fixes the list of commands when the app is compiled, so these exist even when the module is off. They then
//! answer with an error, which the page gets as a rejected promise.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::{
    MODULE, ShelfState,
    items::{Shelf, ShelfItem},
};

/// What is on the shelf now. Files that have gone since the last look are dropped first.
#[tauri::command]
pub fn shelf_items(app: AppHandle) -> Result<Vec<ShelfItem>, String> {
    with_shelf(&app, |shelf| {
        shelf.refresh();
        shelf.items().to_vec()
    })
}

#[tauri::command]
pub fn shelf_add(app: AppHandle, paths: Vec<PathBuf>) -> Result<Vec<ShelfItem>, String> {
    with_shelf(&app, |shelf| {
        shelf.add(paths);
        shelf.items().to_vec()
    })
}

/// Takes an item off the shelf. The file stays where it is.
#[tauri::command]
pub fn shelf_remove(app: AppHandle, id: u64) -> Result<Vec<ShelfItem>, String> {
    with_shelf(&app, |shelf| {
        shelf.remove(id);
        shelf.items().to_vec()
    })
}

/// Does something with the shelf and gives the result, or an error when the module does not run and there is no
/// shelf. `action` is a closure: a piece of code handed over to be run here, once the shelf is at hand.
fn with_shelf<T>(app: &AppHandle, action: impl FnOnce(&mut Shelf) -> T) -> Result<T, String> {
    let Some(state) = app.try_state::<ShelfState>() else {
        return Err(format!("The {} module is off.", MODULE.name));
    };
    let mut shelf = state.lock().unwrap();
    Ok(action(&mut shelf))
}
