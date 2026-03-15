use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{
    window::Color, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::{app_log_info, app_log_warn};

pub const QUICK_WINDOW_LABEL: &str = "quick";
pub const MAIN_WINDOW_LABEL: &str = "main";
pub const QUICK_PANEL_FOCUS_EVENT: &str = "quick_panel:focus_search";
pub const OPEN_FULL_APP_PAYLOAD_EVENT: &str = "quick_panel:open_full_app_payload";

const QUICK_WINDOW_WIDTH: f64 = 920.0;
const QUICK_WINDOW_HEIGHT: f64 = 128.0;
const MAIN_WINDOW_WIDTH: f64 = 1200.0;
const MAIN_WINDOW_HEIGHT: f64 = 800.0;
const MAIN_WINDOW_MIN_WIDTH: f64 = 800.0;
const MAIN_WINDOW_MIN_HEIGHT: f64 = 800.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenFullAppPayload {
    pub query: Option<String>,
    pub selected_path: Option<String>,
    pub timestamp: Option<f64>,
    pub source: Option<String>,
    pub semantic_file_type_filter: Option<String>,
}

#[derive(Default)]
pub struct FullAppHandoffState(pub Mutex<Option<OpenFullAppPayload>>);

#[derive(Default)]
pub struct WindowMetricsState(pub Mutex<WindowMetrics>);

#[derive(Default)]
pub struct WindowMetrics {
    pub quick_panel_opens: u64,
    pub full_app_escalations: u64,
}

pub fn ensure_quick_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(QUICK_WINDOW_LABEL) {
        return Ok(window);
    }

    WebviewWindowBuilder::new(app, QUICK_WINDOW_LABEL, WebviewUrl::App("/".into()))
        .title("Cosmos Quick Panel")
        .inner_size(QUICK_WINDOW_WIDTH, QUICK_WINDOW_HEIGHT)
        .min_inner_size(700.0, 110.0)
        .visible(false)
        .resizable(true)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .background_color(Color(0, 0, 0, 0))
        .always_on_top(true)
        .skip_taskbar(true)
        .maximizable(false)
        .minimizable(false)
        .center()
        .build()
        .map_err(|e| format!("Failed to create quick panel window: {}", e))
}

pub fn ensure_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::App("/".into()))
        .title("Cosmos")
        .inner_size(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT)
        .min_inner_size(MAIN_WINDOW_MIN_WIDTH, MAIN_WINDOW_MIN_HEIGHT)
        .resizable(true)
        .visible(true)
        .center()
        .build()
        .map_err(|e| format!("Failed to create main window: {}", e))
}

fn increment_quick_panel_open_count(app: &AppHandle) {
    if let Some(metrics_state) = app.try_state::<WindowMetricsState>() {
        if let Ok(mut metrics) = metrics_state.0.lock() {
            metrics.quick_panel_opens += 1;
            app_log_info!(
                "📊 QUICK PANEL: opens={}, full_app_escalations={}",
                metrics.quick_panel_opens,
                metrics.full_app_escalations
            );
        }
    }
}

fn increment_full_app_escalation_count(app: &AppHandle) {
    if let Some(metrics_state) = app.try_state::<WindowMetricsState>() {
        if let Ok(mut metrics) = metrics_state.0.lock() {
            metrics.full_app_escalations += 1;
            app_log_info!(
                "📊 QUICK PANEL: opens={}, full_app_escalations={}",
                metrics.quick_panel_opens,
                metrics.full_app_escalations
            );
        }
    }
}

pub fn show_quick_panel_internal(app: &AppHandle) -> Result<(), String> {
    let started_at = Instant::now();
    let quick_window = ensure_quick_window(app)?;

    quick_window
        .show()
        .map_err(|e| format!("Failed to show quick panel: {}", e))?;
    let _ = quick_window.unminimize();
    quick_window
        .set_focus()
        .map_err(|e| format!("Failed to focus quick panel: {}", e))?;
    quick_window
        .emit(QUICK_PANEL_FOCUS_EVENT, ())
        .map_err(|e| format!("Failed to emit quick panel focus event: {}", e))?;

    increment_quick_panel_open_count(app);
    app_log_info!(
        "⚡ QUICK PANEL OPEN: latency={}ms",
        started_at.elapsed().as_millis()
    );
    Ok(())
}

pub fn hide_quick_panel_internal(app: &AppHandle) -> Result<(), String> {
    if let Some(quick_window) = app.get_webview_window(QUICK_WINDOW_LABEL) {
        quick_window
            .hide()
            .map_err(|e| format!("Failed to hide quick panel: {}", e))?;
    }
    Ok(())
}

pub fn toggle_quick_panel_internal(app: &AppHandle) -> Result<(), String> {
    let quick_window = ensure_quick_window(app)?;
    let is_visible = quick_window
        .is_visible()
        .map_err(|e| format!("Failed to query quick panel visibility: {}", e))?;

    if is_visible {
        hide_quick_panel_internal(app)
    } else {
        show_quick_panel_internal(app)
    }
}

pub fn open_full_app_internal(
    app: &AppHandle,
    payload: Option<OpenFullAppPayload>,
) -> Result<(), String> {
    let main_window = ensure_main_window(app)?;
    main_window
        .show()
        .map_err(|e| format!("Failed to show full app window: {}", e))?;
    let _ = main_window.unminimize();
    main_window
        .set_focus()
        .map_err(|e| format!("Failed to focus full app window: {}", e))?;

    if let Some(payload) = payload {
        if let Some(handoff_state) = app.try_state::<FullAppHandoffState>() {
            if let Ok(mut pending_payload) = handoff_state.0.lock() {
                *pending_payload = Some(payload.clone());
            }
        }

        if let Err(e) = main_window.emit(OPEN_FULL_APP_PAYLOAD_EVENT, &payload) {
            app_log_warn!(
                "⚠️ FULL APP HANDOFF: Failed to emit payload event immediately: {}",
                e
            );
        }
        increment_full_app_escalation_count(app);
    }

    hide_quick_panel_internal(app)?;
    Ok(())
}

#[tauri::command]
pub fn show_quick_panel(app: AppHandle) -> Result<(), String> {
    show_quick_panel_internal(&app)
}

#[tauri::command]
pub fn hide_quick_panel(app: AppHandle) -> Result<(), String> {
    hide_quick_panel_internal(&app)
}

#[tauri::command]
pub fn toggle_quick_panel(app: AppHandle) -> Result<(), String> {
    toggle_quick_panel_internal(&app)
}

#[tauri::command]
pub fn focus_quick_search_input(app: AppHandle) -> Result<(), String> {
    let quick_window = ensure_quick_window(&app)?;
    quick_window
        .emit(QUICK_PANEL_FOCUS_EVENT, ())
        .map_err(|e| format!("Failed to emit quick panel focus event: {}", e))
}

#[tauri::command]
pub fn open_full_app(payload: Option<OpenFullAppPayload>, app: AppHandle) -> Result<(), String> {
    open_full_app_internal(&app, payload)
}

#[tauri::command]
pub fn consume_full_app_handoff(app: AppHandle) -> Result<Option<OpenFullAppPayload>, String> {
    if let Some(handoff_state) = app.try_state::<FullAppHandoffState>() {
        return handoff_state
            .0
            .lock()
            .map(|mut pending_payload| pending_payload.take())
            .map_err(|_| "Failed to lock full-app handoff state".to_string());
    }

    Ok(None)
}
