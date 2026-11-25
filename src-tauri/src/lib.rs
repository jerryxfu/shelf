//! Shelf's core: the tray icon, the shelf and its panel. The interface is the web page in `../src`.

mod commands;
#[cfg(feature = "dev-control")]
mod dev;
mod panel;
mod shelf;
mod tray;

pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_drag::init());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .manage(commands::ShelfState::default())
        .manage(panel::Open::default())
        .invoke_handler(tauri::generate_handler![
            commands::shelf_items,
            commands::shelf_add,
            commands::shelf_remove,
            commands::panel_hide,
        ])
        .setup(|app| {
            // No Dock icon and no menu bar of its own: Shelf lives in the menu bar's icons. `Info.plist` says the
            // same for the packaged app, so that no Dock icon flashes while it starts.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

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
