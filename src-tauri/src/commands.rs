use crate::app::{AppState, TimerView, VerdictEvent, emit_tick};
use crate::settings::Settings;
use crate::store::Stats;
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
pub fn start_focus(app: AppHandle, state: St, task: String) -> TimerView {
    state.start_focus(task.trim().to_string());
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn start_break(app: AppHandle, state: St) -> TimerView {
    state.start_break(false);
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn pause(app: AppHandle, state: St) -> TimerView {
    state.set_user_pause(true);
    emit_tick(&app, &state);
    state.view()
}

#[tauri::command]
pub fn resume(app: AppHandle, state: St) -> TimerView {
    state.set_user_pause(false);
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
    crate::llm::ping(&model).await.map_err(|e| e.to_string())
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
}

#[tauri::command]
pub fn check_permissions() -> Permissions {
    #[cfg(target_os = "macos")]
    let screen_recording = unsafe { CGPreflightScreenCaptureAccess() };
    #[cfg(not(target_os = "macos"))]
    let screen_recording = true;
    Permissions { screen_recording }
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
        _ => return Err("未知设置页".into()),
    };
    std::process::Command::new("open")
        .arg(format!("x-apple.systempreferences:com.apple.preference.security?{anchor}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
