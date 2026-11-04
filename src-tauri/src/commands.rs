//! IPC command handlers (contracts/tauri-ipc.md). The token goes in, never out.
// Tauri command arguments must be owned.
#![allow(clippy::needless_pass_by_value)]

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::AppCtx;
use crate::app_state::{apply_tab_edits, emit_view};
use crate::config::is_gitlab_url;
use crate::engine::notifier::tab_with_item;
use crate::engine::poll::{clear_tab_markers, recompute_counts};
use crate::error::{AppError, AppResult};
use crate::gitlab::GitLabClient;
use crate::gitlab::user::fetch_current_user;
use crate::model::{
    ALLOWED_INTERVALS, AppView, NotificationEntry, NotificationKind, OpenTarget, Tab, UrlCheck,
    UserInfo, default_tabs,
};
use crate::secrets;
use crate::url_map::check_url;

#[tauri::command]
pub async fn get_view(ctx: State<'_, AppCtx>) -> AppResult<AppView> {
    Ok(ctx.core.lock().await.build_view())
}

async fn verify(ctx: &AppCtx, token: &str) -> AppResult<UserInfo> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AppError::Invalid("Paste a personal access token".into()));
    }
    fetch_current_user(&GitLabClient::new(ctx.http.clone(), token)).await
}

#[tauri::command]
pub async fn test_token(ctx: State<'_, AppCtx>, token: String) -> AppResult<UserInfo> {
    verify(&ctx, &token).await
}

/// Tests, then stores the token in the Keychain and starts a poll (FR-006).
#[tauri::command]
pub async fn save_token(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    token: String,
) -> AppResult<UserInfo> {
    let user = verify(&ctx, &token).await?;
    let token = token.trim().to_string();
    let to_store = token.clone();
    tauri::async_runtime::spawn_blocking(move || secrets::set_token(&to_store))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
    {
        let mut c = ctx.core.lock().await;
        let changed_user = c.settings.user.as_ref().map(|u| &u.username) != Some(&user.username);
        if changed_user {
            let g = &mut c.state.global;
            g.mentions_baseline_at = None;
            g.mentions_user = None;
            g.seen_mention_ids.clear();
        }
        c.token = Some(token);
        c.settings.user = Some(user.clone());
        c.global_error = None;
        c.save_settings();
        c.save_state();
        emit_view(&app, &c);
    }
    ctx.poller.refresh();
    Ok(user)
}

#[tauri::command]
pub async fn remove_token(app: AppHandle, ctx: State<'_, AppCtx>) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(secrets::delete_token)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
    let mut c = ctx.core.lock().await;
    c.token = None;
    c.settings.user = None;
    c.global_error = Some(AppError::NoToken.to_issue());
    c.save_settings();
    emit_view(&app, &c);
    let _ = app.emit("nav://settings", ());
    Ok(())
}

#[tauri::command]
pub async fn set_interval(app: AppHandle, ctx: State<'_, AppCtx>, minutes: u8) -> AppResult<()> {
    if !ALLOWED_INTERVALS.contains(&minutes) {
        return Err(AppError::Invalid(
            "The interval must be 1, 2, 5 or 10 minutes".into(),
        ));
    }
    let mut c = ctx.core.lock().await;
    c.settings.interval_minutes = minutes;
    c.save_settings();
    ctx.poller.set_interval(minutes);
    emit_view(&app, &c);
    Ok(())
}

#[tauri::command]
pub async fn set_notify_mentions(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    enabled: bool,
) -> AppResult<()> {
    let mut c = ctx.core.lock().await;
    if enabled && !c.settings.notify_mentions {
        // Start a fresh baseline so mentions from while it was off don't flood in.
        c.state.global.mentions_baseline_at = None;
        c.save_state();
    }
    c.settings.notify_mentions = enabled;
    c.save_settings();
    emit_view(&app, &c);
    Ok(())
}

#[tauri::command]
pub fn validate_url(url: String) -> UrlCheck {
    check_url(&url)
}

async fn replace_tabs(app: &AppHandle, ctx: &AppCtx, tabs: Vec<Tab>) -> AppResult<Vec<Tab>> {
    let saved = {
        let mut c = ctx.core.lock().await;
        let saved = apply_tab_edits(tabs, &mut c.state).map_err(AppError::Invalid)?;
        c.settings.tabs.clone_from(&saved);
        if c.active_tab_id
            .as_ref()
            .is_none_or(|id| !saved.iter().any(|t| &t.id == id))
        {
            c.active_tab_id = saved.first().map(|t| t.id.clone());
        }
        let list = c.settings.tabs.clone();
        recompute_counts(&mut c.state, &list);
        c.save_settings();
        c.save_state();
        emit_view(app, &c);
        saved
    };
    ctx.poller.refresh();
    Ok(saved)
}

#[tauri::command]
pub async fn save_tabs(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    tabs: Vec<Tab>,
) -> AppResult<Vec<Tab>> {
    replace_tabs(&app, &ctx, tabs).await
}

#[tauri::command]
pub async fn reset_default_tabs(app: AppHandle, ctx: State<'_, AppCtx>) -> AppResult<Vec<Tab>> {
    replace_tabs(&app, &ctx, default_tabs()).await
}

#[tauri::command]
pub fn refresh_now(ctx: State<'_, AppCtx>) {
    ctx.poller.refresh();
}

/// Sets the active tab and clears its markers (FR-023).
#[tauri::command]
pub async fn select_tab(app: AppHandle, ctx: State<'_, AppCtx>, tab_id: String) -> AppResult<()> {
    let mut c = ctx.core.lock().await;
    if !c.settings.tabs.iter().any(|t| t.id == tab_id) {
        return Err(AppError::Invalid("Unknown tab".into()));
    }
    c.active_tab_id = Some(tab_id.clone());
    if c.window_visible {
        clear_tab_markers(&mut c.state, &tab_id);
    }
    let list = c.settings.tabs.clone();
    recompute_counts(&mut c.state, &list);
    c.save_state();
    emit_view(&app, &c);
    Ok(())
}

/// Entries newest first; marks them all read (FR-024c).
#[tauri::command]
pub async fn open_bell(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
) -> AppResult<Vec<NotificationEntry>> {
    let mut c = ctx.core.lock().await;
    let entries = c.log.entries.clone();
    if c.log.unread_count() > 0 {
        c.log.mark_all_read();
        c.save_log();
        emit_view(&app, &c);
    }
    Ok(entries)
}

/// Row when the item is in a tab, else the browser (FR-024d).
#[tauri::command]
pub async fn open_entry(
    app: AppHandle,
    ctx: State<'_, AppCtx>,
    entry_id: String,
) -> AppResult<OpenTarget> {
    let c = ctx.core.lock().await;
    let entry = c
        .log
        .get(&entry_id)
        .cloned()
        .ok_or_else(|| AppError::Invalid("Unknown entry".into()))?;
    let order: Vec<String> = c.settings.tabs.iter().map(|t| t.id.clone()).collect();
    if let Some(gid) = &entry.item_gid
        && let Some(tab_id) = tab_with_item(&c.state, &order, gid, entry.tab_id.as_deref())
    {
        return Ok(OpenTarget::Row {
            tab_id,
            gid: gid.clone(),
        });
    }
    if entry.kind == NotificationKind::NewItems
        && let Some(tab_id) = entry.tab_id.filter(|id| order.contains(id))
    {
        return Ok(OpenTarget::Tab { tab_id });
    }
    drop(c);
    match entry.web_url {
        Some(url) if is_gitlab_url(&url) => {
            open_in_browser(&app, &url)?;
            Ok(OpenTarget::Browser)
        }
        _ => Ok(OpenTarget::None),
    }
}

fn open_in_browser(app: &AppHandle, url: &str) -> AppResult<()> {
    if !is_gitlab_url(url) {
        return Err(AppError::Invalid(
            "Only gitlab.com links can be opened".into(),
        ));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> AppResult<()> {
    open_in_browser(&app, &url)
}

/// Hides the window, same as clicking outside it (FR-002c).
#[tauri::command]
pub fn hide_window(app: AppHandle) {
    crate::tray::hide_window(&app);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
