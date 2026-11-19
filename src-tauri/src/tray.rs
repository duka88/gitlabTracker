//! Menu bar icon, menu, badge and window toggling (research R1).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::Utc;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::app_state::{Core, emit_view, shared};
use crate::engine::poll::{clear_tab_markers, recompute_counts};

pub const TRAY_ID: &str = "main-tray";

/// A click on the tray that also took focus from the window must not re-open it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

/// True once the window has been placed under the tray icon; later shows keep its position.
static PLACED: AtomicBool = AtomicBool::new(false);

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit GitLab Tracker", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &refresh, &sep, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?)
        .icon_as_template(true)
        .tooltip("GitLab Tracker")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_window(app),
            "refresh" => app.state::<crate::AppCtx>().poller.refresh(),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(app);
            }
        })
        .build(app)?;
    Ok(())
}

/// Menu bar count as title text; empty at 0 (FR-004).
pub fn set_badge(app: &AppHandle, count: u32) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let title = (count > 0).then(|| count.to_string());
        let _ = tray.set_title(title);
    }
}

fn toggle_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    // The window stays open in the background (FR-002d): the icon hides it only when it's
    // in front, and otherwise brings it forward.
    if win.is_visible().unwrap_or(false) && win.is_focused().unwrap_or(false) {
        hide_window(app);
        return;
    }
    let core = shared(app);
    let recently_hidden = core
        .try_lock()
        .ok()
        .and_then(|c| c.window_hidden_at)
        .is_some_and(|t| t.elapsed() < REOPEN_GUARD);
    if !recently_hidden {
        show_window(app);
    }
}

/// Shows the window under the icon and clears the active tab's markers (FR-023).
pub fn show_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    // Snap under the icon until that has worked once; after that the window keeps the
    // position and size the user dragged/resized it to (FR-002). Before the first tray
    // event the icon position is unknown (first run): center instead.
    if !PLACED.load(Ordering::Relaxed) {
        if win.move_window(Position::TrayBottomCenter).is_ok() {
            keep_on_screen(&win);
            PLACED.store(true, Ordering::Relaxed);
        } else {
            let _ = win.center();
        }
    }
    let _ = win.show();
    let _ = win.set_focus();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let core = shared(&app);
        let mut c = core.lock().await;
        mark_window_seen(&app, &mut c);
    });
}

/// Shifts the window left/right so it fits on its monitor; a wide window centered under an
/// icon near the screen edge would otherwise hang off it.
fn keep_on_screen(win: &tauri::WebviewWindow) {
    let (Ok(Some(mon)), Ok(pos), Ok(size)) = (
        win.current_monitor(),
        win.outer_position(),
        win.outer_size(),
    ) else {
        return;
    };
    let left = mon.position().x;
    let right = left + mon.size().width as i32;
    let x = pos.x.min(right - size.width as i32).max(left);
    if x != pos.x {
        let _ = win.set_position(tauri::PhysicalPosition::new(x, pos.y));
    }
}

/// The user is looking at the window again (shown, or refocused while open): the active
/// tab counts as opened, so its markers clear (FR-023, FR-002d).
pub fn mark_window_seen(app: &AppHandle, c: &mut Core) {
    c.window_visible = true;
    c.window_shown_at = Utc::now();
    if let Some(id) = c.active_tab() {
        clear_tab_markers(&mut c.state, &id);
    }
    let tabs = c.settings.tabs.clone();
    recompute_counts(&mut c.state, &tabs);
    c.save_state();
    emit_view(app, c);
}

pub fn hide_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let core = shared(&app);
        let mut c = core.lock().await;
        c.window_visible = false;
        c.window_hidden_at = Some(Instant::now());
    });
}
