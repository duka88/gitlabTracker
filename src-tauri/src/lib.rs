pub mod app_state;
pub mod avatars;
pub mod commands;
pub mod config;
pub mod engine;
pub mod error;
pub mod gitlab;
pub mod model;
pub mod secrets;
pub mod store;
pub mod text;
pub mod tray;
pub mod url_map;

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tauri::{Emitter, Listener, Manager, RunEvent};

use app_state::{Core, Paths, Shared, shared};
use avatars::AvatarCache;
use engine::notifier::{ClickRoute, TauriNotifier, route_click};
use engine::poller::{self, PollerHandle};

/// Managed app context.
pub struct AppCtx {
    pub core: Shared,
    pub poller: PollerHandle,
    pub http: reqwest::Client,
}

/// A click on a notification activates the app within this window (research R9).
const CLICK_WINDOW: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Visibility {
    visible: bool,
    active_tab_id: Option<String>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http = gitlab::http_client();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_positioner::init())
        .register_asynchronous_uri_scheme_protocol("avatar", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_string();
            tauri::async_runtime::spawn(async move {
                let cache = app.state::<Arc<AvatarCache>>().inner().clone();
                responder.respond(cache.serve(&path).await);
            });
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();

            let cache_dir = app.path().app_cache_dir()?.join("avatars");
            app.manage(Arc::new(AvatarCache::new(cache_dir, http.clone())));

            let mut core = Core::load(Paths::in_dir(&app.path().app_data_dir()?));
            core.token = secrets::get_token().unwrap_or_else(|e| {
                eprintln!("[gitlab-tracker] {e}");
                None
            });
            let needs_token = core.token.is_none() || core.settings.user.is_none();
            let interval = core.settings.interval_minutes;
            let core: Shared = Arc::new(tokio::sync::Mutex::new(core));
            let notifier = Arc::new(TauriNotifier::new(handle.clone()));
            let poller = poller::spawn(
                handle.clone(),
                core.clone(),
                http.clone(),
                notifier,
                interval,
            );
            app.manage(AppCtx {
                core,
                poller,
                http: http.clone(),
            });

            tray::setup(&handle)?;

            let h = handle.clone();
            app.listen("ui://visibility", move |event| {
                let Ok(v) = serde_json::from_str::<Visibility>(event.payload()) else {
                    return;
                };
                let h = h.clone();
                tauri::async_runtime::spawn(async move {
                    let core = shared(&h);
                    let mut c = core.lock().await;
                    let became_visible = v.visible && !c.window_visible;
                    c.window_visible = v.visible;
                    if let Some(id) = v.active_tab_id
                        && c.settings.tabs.iter().any(|t| t.id == id)
                    {
                        c.active_tab_id = Some(id);
                    }
                    if became_visible {
                        tray::mark_window_seen(&h, &mut c);
                    }
                });
            });

            // First run without a token: open Settings (FR-007).
            if needs_token {
                tray::show_window(&handle);
                let _ = handle.emit("nav://settings", ());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::test_token,
            commands::save_token,
            commands::remove_token,
            commands::set_interval,
            commands::set_notify_mentions,
            commands::set_test_branch,
            commands::set_tracked_envs,
            commands::validate_url,
            commands::save_tabs,
            commands::reset_default_tabs,
            commands::refresh_now,
            commands::select_tab,
            commands::open_bell,
            commands::open_entry,
            commands::open_url,
            commands::hide_window,
            commands::quit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        if let RunEvent::Reopen { .. } = event {
            on_activate(app);
        }
    });
}

/// App activation (e.g. a notification click): open the window on the row or the bell (FR-024).
fn on_activate(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let core = shared(&app);
        let route = {
            let c = core.lock().await;
            let recent = c
                .last_notification_at
                .is_some_and(|t| t.elapsed() < CLICK_WINDOW);
            let order: Vec<String> = c.settings.tabs.iter().map(|t| t.id.clone()).collect();
            recent.then(|| route_click(&c.log, &c.state, &order, c.window_shown_at))
        };
        tray::show_window(&app);
        match route {
            Some(ClickRoute::Row(target)) => {
                let _ = app.emit("nav://row", target);
            }
            Some(ClickRoute::Bell) => {
                let _ = app.emit("nav://bell", ());
            }
            None => {}
        }
    });
}
