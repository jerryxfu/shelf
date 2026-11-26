//! The shelf's panel: a small window that floats above the others.
//!
//! It is created when shown and destroyed when hidden, so a hidden shelf has no web view: only this process runs.
//! Everything here must be called on the main thread.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, LogicalPosition, Manager};

use crate::{modules::shelf, tray};

pub const LABEL: &str = "shelf";

const WIDTH: f64 = 300.0;
const HEIGHT: f64 = 380.0;
/// Space left between the panel and the tray icon, and between the panel and the screen's edges.
const GAP: f64 = 6.0;

/// Whether the shelf is open. Kept here and not asked of Tauri: a window that was just destroyed stays in Tauri's
/// list until the event loop gets to it, so right after hiding it would still count as open.
#[derive(Default)]
pub struct Open(AtomicBool);

pub fn is_open(app: &AppHandle) -> bool {
    app.state::<Open>().0.load(Ordering::Relaxed)
}

pub fn toggle(app: &AppHandle) {
    if is_open(app) { hide(app) } else { show(app) }
}

pub fn show(app: &AppHandle) {
    // The panel only shows the shelf so far: when the Shelf module does not run, there is nothing to open.
    if is_open(app) || !shelf::is_running(app) {
        return;
    }
    match platform::create(app, position(app)) {
        Ok(()) => set_open(app, true),
        Err(error) => eprintln!("could not open the shelf: {error}"),
    }
}

pub fn hide(app: &AppHandle) {
    platform::destroy(app);
    set_open(app, false);
}

fn set_open(app: &AppHandle, open: bool) {
    app.state::<Open>().0.store(open, Ordering::Relaxed);
    tray::sync(app);
}

/// Next to the tray icon: under it when the icon is in the top half of its screen (the macOS menu bar), above it
/// otherwise (the Windows taskbar), and always whole on that screen. `None` leaves the placement to the system.
fn position(app: &AppHandle) -> Option<LogicalPosition<f64>> {
    let icon = tray::rect(app)?;
    let icon_at = icon.position.to_physical::<f64>(1.0);
    let icon_size = icon.size.to_physical::<f64>(1.0);
    // The screen the icon is on. Not `monitor_from_point`: on macOS it compares against the screens in points, and
    // the icon's place is in pixels, so on a Retina screen it finds nothing.
    let monitor = app.available_monitors().ok()?.into_iter().find(|monitor| {
        let x = icon_at.x - f64::from(monitor.position().x);
        let y = icon_at.y - f64::from(monitor.position().y);
        (0.0..f64::from(monitor.size().width)).contains(&x) && (0.0..f64::from(monitor.size().height)).contains(&y)
    })?;

    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (left, top) = (f64::from(area.position.x), f64::from(area.position.y));
    let (right, bottom) = (left + f64::from(area.size.width), top + f64::from(area.size.height));
    let (width, height, gap) = (WIDTH * scale, HEIGHT * scale, GAP * scale);

    let x = icon_at.x + icon_size.width / 2.0 - width / 2.0;
    let y = if icon_at.y < (top + bottom) / 2.0 {
        icon_at.y + icon_size.height + gap
    } else {
        icon_at.y - height - gap
    };
    let x = x.min(right - width - gap).max(left + gap);
    let y = y.min(bottom - height - gap).max(top + gap);

    Some(LogicalPosition::new(x / scale, y / scale))
}

#[cfg(target_os = "macos")]
mod platform {
    use tauri::{
        AppHandle, LogicalPosition, LogicalSize, Position, Size, WebviewUrl,
        window::{Effect, EffectState, EffectsBuilder},
    };
    use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask, tauri_panel};

    use super::{HEIGHT, LABEL, WIDTH};

    /// The corners' radius. The interface rounds its own corners to match (`$radius` in `ShelfPanel.scss`).
    const RADIUS: f64 = 12.0;

    // An NSPanel instead of Tauri's NSWindow: with the non-activating style below, a click in the panel does not
    // activate Shelf, so the app in use keeps its menu bar and its place in front. The panel takes the keyboard
    // only when something in it needs typing, which nothing does yet.
    tauri_panel! {
        panel!(ShelfPanel {
            config: {
                can_become_key_window: true,
                can_become_main_window: false,
                becomes_key_only_if_needed: true,
                is_floating_panel: true
            }
        })
    }

    pub fn create(app: &AppHandle, position: Option<LogicalPosition<f64>>) -> tauri::Result<()> {
        let mut builder = PanelBuilder::<_, ShelfPanel>::new(app, LABEL)
            .url(WebviewUrl::App("index.html".into()))
            .title("Shelf")
            .size(Size::Logical(LogicalSize::new(WIDTH, HEIGHT)))
            .level(PanelLevel::Floating)
            .has_shadow(true)
            .hides_on_deactivate(false)
            // On every desktop, and over an app in full screen.
            .collection_behavior(CollectionBehavior::new().can_join_all_spaces().full_screen_auxiliary())
            .add_style_mask(StyleMask::empty().nonactivating_panel())
            .no_activate(true)
            .with_window(|window| {
                window
                    .decorations(false)
                    .resizable(false)
                    // Shown below, once it is a panel.
                    .visible(false)
                    // A panel that is not the key window still gets the first click, not only the second.
                    .accept_first_mouse(true)
                    // The system's own popover material behind the interface, with rounded corners.
                    .transparent(true)
                    .effects(
                        EffectsBuilder::new()
                            .effect(Effect::Popover)
                            .state(EffectState::Active)
                            .radius(RADIUS)
                            .build(),
                    )
            });
        if let Some(position) = position {
            builder = builder.position(Position::Logical(position));
        }

        // In front of the other windows, without activating Shelf.
        builder.build()?.show();
        Ok(())
    }

    pub fn destroy(app: &AppHandle) {
        // Back to a plain window first, so that Tauri can close it as one of its own.
        if let Some(window) = app.get_webview_panel(LABEL).ok().and_then(|panel| panel.to_window()) {
            let _ = window.destroy();
        }
    }
}

// Written from the documentation. It compiles for Windows (`cargo check --target x86_64-pc-windows-msvc`), but it
// has never run: Windows could not be tested on the Mac this was built on.
#[cfg(not(target_os = "macos"))]
mod platform {
    use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder};

    use super::{HEIGHT, LABEL, WIDTH};

    pub fn create(app: &AppHandle, position: Option<LogicalPosition<f64>>) -> tauri::Result<()> {
        let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
            .title("Shelf")
            .inner_size(WIDTH, HEIGHT)
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            // Not focused when it opens, and a click in it does not take the focus (WS_EX_NOACTIVATE on Windows).
            .focused(false)
            .focusable(false);
        if let Some(position) = position {
            builder = builder.position(position.x, position.y);
        }

        builder.build()?;
        Ok(())
    }

    pub fn destroy(app: &AppHandle) {
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.destroy();
        }
    }
}
