//! What a tick looks and sounds like outside the windows: the menu-bar item, sounds, notifications.
use super::{AppState, TimerView};
use crate::timer::Phase;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

pub fn play(sound: &str) {
    let _ = std::process::Command::new("afplay").arg(format!("/System/Library/Sounds/{sound}.aiff")).spawn();
}

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn fmt_mmss(secs: u32) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TrayLook {
    Idle,
    Focus,
    Paused,
}

static TRAY_LOOK: Mutex<Option<TrayLook>> = Mutex::new(None);

fn update_tray(app: &AppHandle, v: &TimerView) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let title = match v.phase {
        // A finished round or break nobody answered: count up, so the overrun is in plain sight.
        Phase::Idle if v.overtime > 0 => format!(" +{}", fmt_mmss(v.overtime)),
        Phase::Idle => String::new(),
        // Drifting: a quiet mark by the clock before (and while) the reminder is up.
        Phase::Focus if v.paused.is_none() && (v.nudging || v.drifting) => format!(" {} ⚠\u{FE0E}", fmt_mmss(v.remaining)),
        _ => format!(" {}", fmt_mmss(v.remaining)),
    };
    let _ = tray.set_title(Some(title));
    let look = match (v.phase, v.paused) {
        (Phase::Idle, _) => TrayLook::Idle,
        (_, Some(_)) => TrayLook::Paused,
        (Phase::Focus, None) => TrayLook::Focus,
        (_, None) => TrayLook::Idle,
    };
    let mut cur = TRAY_LOOK.lock().unwrap();
    if *cur == Some(look) {
        return;
    }
    *cur = Some(look);
    // The focus icon is the tomato in colour; the others are template icons that follow the menu bar.
    let (img, template) = match look {
        TrayLook::Idle => (tauri::include_image!("icons/tray.png"), true),
        TrayLook::Focus => (tauri::include_image!("icons/tray-focus.png"), false),
        TrayLook::Paused => (tauri::include_image!("icons/tray-paused.png"), true),
    };
    let _ = tray.set_icon(Some(img));
    let _ = tray.set_icon_as_template(template);
}

pub fn emit_tick(app: &AppHandle, state: &AppState) {
    let v = state.view();
    update_tray(app, &v);
    let _ = app.emit("timer://tick", &v);
}
