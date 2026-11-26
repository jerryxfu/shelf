//! The Shelf module: a drop spot that holds files until the user acts on them.
//!
//! - `items.rs` is the shelf itself, in plain Rust.
//! - `commands.rs` is what the interface may ask of it.
//!
//! This file says what the module is, and starts it.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use super::{Module, System};

pub mod commands;
mod items;

pub const MODULE: Module = Module {
    id: "shelf",
    name: "Shelf",
    systems: &[System::MacOs, System::Windows],
};

/// The shelf, shared by the commands. A `Mutex` lets only one of them touch it at a time.
type ShelfState = Mutex<items::Shelf>;

/// Starts the module: from here on there is a shelf to put files on.
///
/// When the module does not run this is never called, and nothing of the shelf exists: no list of files, no entry
/// in the tray menu, no panel.
pub fn start(app: &AppHandle) {
    app.manage(ShelfState::default());
}

/// Whether the module was started.
pub fn is_running(app: &AppHandle) -> bool {
    app.try_state::<ShelfState>().is_some()
}
