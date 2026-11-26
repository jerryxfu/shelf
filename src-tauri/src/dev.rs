//! A way to drive the app from a script while testing, where no mouse is at hand. Built only with
//! `--features dev-control`, so it is never in a normal build.
//!
//! One command per line on standard input: `show`, `hide`, `toggle`, `state`, `quit`, and `eval <script>`, which runs
//! a script in the panel's page. A script answers by emitting a `dev-log` event, printed here on standard output:
//!
//! ```text
//! eval window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {event: "dev-log", payload: document.title})
//! ```
//!
//! Two things learned the hard way:
//!
//! - Scripts share the page's global scope: wrap one that declares names in `(() => { ... })()`, or the next script
//!   that declares the same name fails without a word.
//! - Never send pointer events to a shelf row from a script. They start a real drag of a real file with no button
//!   held, which then waits for the next mouse event and drops wherever the pointer is. The page's call to the core
//!   cannot be intercepted to prevent it (`__TAURI_INTERNALS__.invoke` is read-only).
//!
//! `pnpm tauri dev` does not pass standard input on to the app: run the built binary itself, with `pnpm dev` running
//! for a debug build.
//!
//! Set `SHELF_DATA_DIR` to a scratch folder and Shelf keeps its settings and its action log there, not in the real
//! place. Do that for any test that changes them.

use std::{io::BufRead, path::PathBuf};

use tauri::{AppHandle, Listener, Manager};

use crate::{modules::shelf, panel};

/// The folder named by `SHELF_DATA_DIR`, if it is set.
pub fn data_dir() -> Option<PathBuf> {
    std::env::var_os("SHELF_DATA_DIR").map(PathBuf::from)
}

pub fn start(app: &AppHandle) {
    app.listen("dev-log", |event| println!("[dev-log] {}", event.payload()));

    let app = app.clone();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || run(&handle, line.trim()));
        }
    });
}

fn run(app: &AppHandle, line: &str) {
    let (command, rest) = line.split_once(' ').unwrap_or((line, ""));
    match command {
        "show" => panel::show(app),
        "hide" => panel::hide(app),
        "toggle" => panel::toggle(app),
        "state" => println!(
            "[dev] {}\n[dev] {}\n[dev] menu says {:?}\n[dev] {}",
            shelf_state(app),
            state(app),
            crate::tray::toggle_label(app),
            tray_state(app)
        ),
        "quit" => app.exit(0),
        "eval" => match app.get_webview_window(panel::LABEL) {
            Some(window) => {
                if let Err(error) = window.eval(rest) {
                    println!("[dev] eval failed: {error}");
                }
            }
            None => println!("[dev] the panel is not open"),
        },
        "" => return,
        other => {
            println!("[dev] unknown command: {other}");
            return;
        }
    }
    println!("[dev] done: {command}");
}

/// Whether the Shelf module runs, and what its `shelf_items` command answers: the names on the shelf, or the error
/// it gives when the module is off. The command is called here as a plain function, with no page in between, so
/// this works with the panel closed.
fn shelf_state(app: &AppHandle) -> String {
    let answer = shelf::commands::shelf_items(app.clone())
        .map(|items| items.into_iter().map(|item| item.name).collect::<Vec<_>>());
    let runs = if shelf::is_running(app) {
        "running"
    } else {
        "not running"
    };
    format!("shelf module: {runs}, shelf_items answers {answer:?}")
}

/// What can be read of the panel without clicking it.
#[cfg(target_os = "macos")]
fn state(app: &AppHandle) -> String {
    use tauri_nspanel::{
        ManagerExt,
        objc2::MainThreadMarker,
        objc2_app_kit::{NSApplication, NSWindowStyleMask},
    };

    let Ok(panel) = app.get_webview_panel(panel::LABEL) else {
        return "panel: closed".into();
    };
    let native = panel.as_panel();
    let frame = native.frame();
    let app_active = MainThreadMarker::new().map(|main| NSApplication::sharedApplication(main).isActive());

    format!(
        "panel: open, class {}, visible {}, non-activating {}, key {}, can become key {}, floating {}, level {}, \
         frame {}x{} at {},{}, app active {:?}",
        native.class().name().to_string_lossy(),
        panel.is_visible(),
        native.styleMask().contains(NSWindowStyleMask::NonactivatingPanel),
        native.isKeyWindow(),
        panel.can_become_key_window(),
        panel.is_floating_panel(),
        native.level(),
        frame.size.width,
        frame.size.height,
        frame.origin.x,
        frame.origin.y,
        app_active,
    )
}

#[cfg(not(target_os = "macos"))]
fn state(app: &AppHandle) -> String {
    format!("panel: {}", if panel::is_open(app) { "open" } else { "closed" })
}

/// Whether the system shows the tray icon, and where.
#[cfg(target_os = "macos")]
fn tray_state(app: &AppHandle) -> String {
    use tauri_nspanel::objc2::MainThreadMarker;

    let Some(tray) = app.tray_by_id(crate::tray::TRAY) else {
        return "tray: none".into();
    };
    let native = tray.with_inner_tray_icon(|inner| {
        let item = inner.ns_status_item()?;
        let button = item.button(MainThreadMarker::new()?)?;
        Some(format!(
            "visible {}, length {}, button has a window {}, button frame {:?}",
            item.isVisible(),
            item.length(),
            button.window().is_some(),
            button.frame(),
        ))
    });
    format!("tray: rect {:?}, native {native:?}", tray.rect())
}

#[cfg(not(target_os = "macos"))]
fn tray_state(app: &AppHandle) -> String {
    format!(
        "tray: rect {:?}",
        app.tray_by_id(crate::tray::TRAY).map(|tray| tray.rect())
    )
}
