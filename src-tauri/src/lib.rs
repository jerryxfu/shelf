//! Shelf's core: the tray icon, the panel, the settings, the action log, and the modules. The interface is the web
//! page in `../src`.
//!
//! This file is the glue: it reads the settings, starts the modules that run, and hands everything to Tauri. The
//! logic is in the other files.

mod commands;
#[cfg(feature = "dev-control")]
mod dev;
mod log;
mod modules;
mod panel;
#[cfg(test)]
mod scratch;
mod settings;
mod tray;

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use tauri::{App, AppHandle, Manager};

use crate::{
    log::ActionLog,
    modules::{System, shelf},
    settings::{Loaded, Settings},
};

pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_drag::init());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .manage(panel::Open::default())
        // Fixed when the app is compiled: a module that is off keeps its commands, and they answer with an error.
        .invoke_handler(tauri::generate_handler![
            commands::panel_hide,
            shelf::commands::shelf_items,
            shelf::commands::shelf_add,
            shelf::commands::shelf_remove,
        ])
        .setup(|app| {
            // No Dock icon and no menu bar of its own: Shelf lives in the menu bar's icons. `Info.plist` says the
            // same for the packaged app, so that no Dock icon flashes while it starts.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data = data_dir(app)?;
            let settings = load_settings(&data);
            // One log for all modules, behind a `Mutex` so that one of them at a time moves files.
            app.manage(Mutex::new(ActionLog::new(&data)));
            start_modules(app.handle(), &settings);

            // After the modules: the menu only has entries for the ones that run.
            tray::create(app.handle())?;

            #[cfg(feature = "dev-control")]
            dev::start(app.handle());

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Shelf could not start")
        .run(|_app, event| {
            // Closing the last window is not quitting: Shelf stays in the tray. A real exit (the menu's Quit) has a
            // code.
            if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}

/// Where Shelf keeps its own files, the settings and the action log: the system's folder for app data.
fn data_dir(app: &App) -> tauri::Result<PathBuf> {
    // A test can send Shelf to a scratch folder, so that it never touches the real settings.
    #[cfg(feature = "dev-control")]
    if let Some(dir) = dev::data_dir() {
        return Ok(dir);
    }
    app.path().app_data_dir()
}

/// Reads the settings. Whatever is wrong with the file, Shelf still starts.
fn load_settings(dir: &Path) -> Settings {
    match settings::load(dir) {
        Loaded::Found(settings) => settings,
        Loaded::Missing => {
            // The first start: the defaults are written, so that there is a file to edit by hand.
            let settings = modules::default_settings();
            if let Err(error) = settings.save(dir) {
                eprintln!("could not write the settings in {}: {error}", dir.display());
            }
            settings
        }
        Loaded::Unreadable(problem) => {
            // The file is left alone, and nothing is written over it.
            eprintln!(
                "could not read {} in {}, so the default settings are used: {problem}",
                settings::FILE,
                dir.display()
            );
            modules::default_settings()
        }
    }
}

/// Starts the modules that run. One that is off, or that does not support this system, is skipped: it starts nothing.
fn start_modules(app: &AppHandle, settings: &Settings) {
    let system = System::current();

    if shelf::MODULE.runs(system, settings) {
        shelf::start(app);
    }
}
