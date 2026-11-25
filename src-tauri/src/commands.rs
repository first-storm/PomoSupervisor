use crate::app::{AppState, TimerView, VerdictEvent, emit_tick};
use crate::settings::Settings;
use crate::store::{Review, Stats};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

type St<'a> = State<'a, Arc<AppState>>;

#[tauri::command]
pub fn get_state(state: St) -> TimerView {
    state.view()
}

#[tauri::command]
pub fn last_verdict(state: St) -> Option<VerdictEvent> {
    state.core.lock().unwrap().last.clone()
}

#[tauri::command]
pub fn start_focus(app: AppHandle, state: St, task: String, minutes: Option<u32>, allow: Option<Vec<String>>) -> TimerView {
    let allow = allow.unwrap_or_default().into_iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
    state.start_focus(task.trim().to_string(), minutes, allow);
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn start_break(app: AppHandle, state: St) -> TimerView {
    state.start_break();
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn rest(app: AppHandle, state: St, minutes: u32) -> TimerView {
    state.rest(minutes);
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn allow_for_round(app: AppHandle, state: St, item: String) -> TimerView {
    state.allow_for_round(&item);
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn dismiss_nudge(app: AppHandle, state: St) -> TimerView {
    state.dismiss_nudge();
    emit_tick(&app, &state);
    state.view()
}

/// Give the popover the keyboard: it was shown without focus and now has something to type into.
#[tauri::command]
pub fn focus_popover(app: AppHandle) {
    crate::windows::show_popover(&app, "normal");
}

/// "其实在工作": whitelist this app or site for good.
#[tauri::command]
pub fn mark_as_work(state: St, app_name: String, url: Option<String>) -> Result<String, String> {
    let target = crate::rules::allow_target(&app_name, url.as_deref())
        .ok_or("读不到这个网页的网址，没法只放行它。先在系统设置里允许读取浏览器网址。")?;
    state.mark_as_work(&target).map_err(|e| e.to_string())?;
    Ok(target)
}

#[tauri::command]
pub fn set_auto_resume(app: AppHandle, state: St, on: bool) -> Result<TimerView, String> {
    let mut s = state.settings();
    s.auto_resume = on;
    s.save(&state.settings_path).map_err(|e| e.to_string())?;
    *state.settings.write().unwrap() = s;
    emit_tick(&app, &state);
    Ok(state.view())
}

#[tauri::command]
pub fn recent_verdicts(state: St) -> Vec<VerdictEvent> {
    state.core.lock().unwrap().recent.iter().cloned().collect()
}

#[tauri::command]
pub fn get_review(state: St, id: Option<i64>) -> Result<Option<Review>, String> {
    let every = state.settings.read().unwrap().sample_secs;
    state.store.lock().unwrap().review(id, every).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn today_completed(state: St) -> Result<i64, String> {
    state.store.lock().unwrap().today_completed().map_err(|e| e.to_string())
}

// ---- windows ----

#[tauri::command]
pub fn open_window(app: AppHandle, label: String) -> Result<(), String> {
    matches!(label.as_str(), "review" | "settings").then_some(()).ok_or("未知窗口")?;
    crate::windows::open(&app, &label);
    Ok(())
}

#[tauri::command]
pub fn close_window(app: AppHandle, window: tauri::WebviewWindow) {
    match window.label() {
        "main" => crate::windows::dismiss_popover(&app),
        // Settings asked to close itself after flushing its last save (see lib.rs).
        _ => {
            let _ = window.destroy();
        }
    }
}

/// The popover sizes itself to its card and stays centred under the tray icon.
#[tauri::command]
pub fn popover_fit(app: AppHandle, width: f64, height: f64) {
    crate::windows::fit_popover(&app, width, height);
}

#[tauri::command]
pub fn popover_menu(window: tauri::WebviewWindow) -> Result<(), String> {
    crate::windows::more_menu(&window).map_err(|e| e.to_string())
}

/// Pausing takes the short break's length; the round carries on by itself afterwards.
#[tauri::command]
pub fn pause(app: AppHandle, state: St) -> TimerView {
    let minutes = state.settings.read().unwrap().short_break_min;
    state.rest(minutes);
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn resume(app: AppHandle, state: St) -> TimerView {
    state.resume();
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn finish_early(app: AppHandle, state: St) -> TimerView {
    state.finish_early();
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn still_here(app: AppHandle, state: St) -> TimerView {
    state.still_here();
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn stop(app: AppHandle, state: St) -> TimerView {
    state.stop();
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn get_settings(state: St) -> Settings {
    state.settings()
}

#[tauri::command(async)]
pub fn save_settings(state: St, settings: Settings) -> Result<Settings, String> {
    let settings = settings.sanitized();
    settings.save(&state.settings_path).map_err(|e| e.to_string())?;
    *state.settings.write().unwrap() = settings.clone();
    state.rebuild_models();
    Ok(settings)
}

#[derive(Serialize)]
pub struct ModelStatus {
    pub text_key: bool,
    pub vision_key: bool,
    pub text_ready: bool,
    pub vision_ready: bool,
    pub text_error: Option<String>,
    pub vision_error: Option<String>,
}

#[tauri::command(async)]
pub fn model_status(state: St) -> ModelStatus {
    let m = state.models.read().unwrap();
    ModelStatus {
        text_key: crate::llm::get_key("text").is_some(),
        vision_key: crate::llm::get_key("vision").is_some(),
        text_ready: m.text.is_some(),
        vision_ready: m.vision.is_some(),
        text_error: m.text_error.clone(),
        vision_error: m.vision_error.clone(),
    }
}

fn slot_ok(slot: &str) -> Result<(), String> {
    matches!(slot, "text" | "vision").then_some(()).ok_or_else(|| "未知模型槽位".to_string())
}

#[tauri::command(async)]
pub fn set_api_key(state: St, slot: String, key: String) -> Result<(), String> {
    slot_ok(&slot)?;
    crate::llm::set_key(&slot, key.trim()).map_err(|e| e.to_string())?;
    state.rebuild_models();
    Ok(())
}

#[tauri::command]
pub async fn test_provider(state: State<'_, Arc<AppState>>, slot: String) -> Result<String, String> {
    slot_ok(&slot)?;
    let (model, err) = {
        let m = state.models.read().unwrap();
        if slot == "text" { (m.text.clone(), m.text_error.clone()) } else { (m.vision.clone(), m.vision_error.clone()) }
    };
    let model = model.ok_or_else(|| err.unwrap_or_else(|| "尚未配置该模型".into()))?;
    if slot == "vision" { crate::llm::ping_image(&model).await } else { crate::llm::ping(&model).await }.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_stats(state: St) -> Result<Stats, String> {
    state.store.lock().unwrap().stats().map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[derive(Serialize)]
pub struct Permissions {
    pub screen_recording: bool,
    pub accessibility: bool,
}

#[tauri::command]
pub fn check_permissions() -> Permissions {
    #[cfg(target_os = "macos")]
    let screen_recording = unsafe { CGPreflightScreenCaptureAccess() };
    #[cfg(not(target_os = "macos"))]
    let screen_recording = true;
    Permissions { screen_recording, accessibility: crate::ax::trusted() }
}

#[tauri::command]
pub fn request_accessibility() -> bool {
    crate::ax::request()
}

#[tauri::command]
pub fn request_screen_recording() -> bool {
    #[cfg(target_os = "macos")]
    return unsafe { CGRequestScreenCaptureAccess() };
    #[cfg(not(target_os = "macos"))]
    true
}

#[tauri::command]
pub fn open_privacy(pane: String) -> Result<(), String> {
    let anchor = match pane.as_str() {
        "screen" => "Privacy_ScreenCapture",
        "automation" => "Privacy_Automation",
        "accessibility" => "Privacy_Accessibility",
        _ => return Err("未知设置页".into()),
    };
    std::process::Command::new("open")
        .arg(format!("x-apple.systempreferences:com.apple.preference.security?{anchor}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
