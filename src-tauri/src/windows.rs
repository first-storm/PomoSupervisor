//! The menu-bar popover and the Review / Settings windows.
use std::sync::atomic::{AtomicI64, Ordering};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// When the popover last hid because it lost focus. A click on the tray icon blurs the
/// popover before the click arrives, so a click right after that hide must not reopen it.
static BLUR_HIDE_MS: AtomicI64 = AtomicI64::new(0);

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn popover(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// A monitor's bounds in logical points (macOS's own coordinate space).
fn logical_bounds(m: &tauri::Monitor) -> (f64, f64, f64, f64) {
    let k = m.scale_factor();
    (m.position().x as f64 / k, m.position().y as f64 / k, m.size().width as f64 / k, m.size().height as f64 / k)
}

/// Centre the popover under the tray icon, kept on screen.
///
/// Everything is done in logical points: with a 2x and a 1x display attached, "physical"
/// positions are each scaled by their own screen's factor and don't share one space. The
/// tray rect is physical at its screen's scale, so find the monitor it falls in when divided
/// by that monitor's scale.
fn place(app: &AppHandle, w: &WebviewWindow) {
    let Ok(size) = w.outer_size() else { return };
    let ws = w.scale_factor().unwrap_or(1.0);
    let (ww, wh) = (size.width as f64 / ws, size.height as f64 / ws);
    let monitors = app.available_monitors().unwrap_or_default();
    let rect = app.tray_by_id("main").and_then(|t| t.rect().ok().flatten());
    let anchored = rect.and_then(|r| {
        monitors.iter().find_map(|m| {
            let k = m.scale_factor();
            let p = r.position.to_logical::<f64>(k);
            let sz = r.size.to_logical::<f64>(k);
            let (cx, cy) = (p.x + sz.width / 2.0, p.y + sz.height / 2.0);
            let (mx, my, mw, mh) = logical_bounds(m);
            (cx >= mx && cx < mx + mw && cy >= my && cy < my + mh).then_some((cx, p.y + sz.height, m))
        })
    });
    let (cx, top, bounds) = match anchored {
        Some((cx, bottom, m)) => (cx, bottom + 6.0, logical_bounds(m)),
        // No tray rect (e.g. hidden by a menu-bar manager): top right of the primary screen.
        None => {
            let Some(m) = w.primary_monitor().ok().flatten() else { return };
            let b = logical_bounds(&m);
            (b.0 + b.2 - ww / 2.0 - 12.0, b.1 + 40.0, b)
        }
    };
    let (mx, my, mw, mh) = bounds;
    let x = (cx - ww / 2.0).clamp(mx + 8.0, (mx + mw - ww - 8.0).max(mx));
    let y = top.clamp(my, (my + mh - wh).max(my));
    let _ = w.set_position(LogicalPosition::new(x.round(), y.round()));
}

/// Make this the active app. A click on a menu-bar item doesn't do that by itself, and
/// without it the popover never gets focus, so clicking elsewhere couldn't hide it.
#[cfg(target_os = "macos")]
fn activate(app: &AppHandle) {
    let _ = app.run_on_main_thread(|| {
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            #[allow(deprecated)]
            objc2_app_kit::NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
        }
    });
}

/// `mode`: "normal" (from the tray: takes focus, hides on blur), or anything else ("nudge",
/// "passive") for a card the app brings up by itself: the distraction reminder, a finished
/// round, a finished break. Those show over whatever is in front without taking the keyboard.
pub fn show_popover(app: &AppHandle, mode: &str) {
    let Some(w) = popover(app) else { return };
    let _ = w.emit_to("main", "popover://mode", mode);
    #[cfg(target_os = "macos")]
    {
        let _ = app.show();
        if mode == "normal" {
            activate(app);
        }
    }
    place(app, &w);
    let _ = w.show();
    if mode == "normal" {
        let _ = w.set_focus();
    }
}

/// Take down a card the app put up by itself. The person never left their own window, so
/// app activation is left alone.
pub fn hide_passive(app: &AppHandle) {
    if let Some(w) = popover(app) {
        let _ = w.hide();
    }
}

/// Hide the popover on purpose (not on blur). With no other window open, also hide the
/// app so macOS hands focus back to whatever the person was using.
pub fn dismiss_popover(app: &AppHandle) {
    let Some(w) = popover(app) else { return };
    let _ = w.hide();
    let others = ["review", "settings"].iter().any(|l| app.get_webview_window(l).is_some());
    #[cfg(target_os = "macos")]
    if !others {
        let _ = app.hide();
    }
    #[cfg(not(target_os = "macos"))]
    let _ = others;
}

pub fn toggle_popover(app: &AppHandle) {
    let Some(w) = popover(app) else { return };
    if w.is_visible().unwrap_or(false) {
        dismiss_popover(app);
    } else if now_ms() - BLUR_HIDE_MS.load(Ordering::Relaxed) > 300 {
        show_popover(app, "normal");
    }
}

pub fn on_popover_blur(w: &tauri::Window) {
    BLUR_HIDE_MS.store(now_ms(), Ordering::Relaxed);
    let _ = w.hide();
}

pub fn fit_popover(app: &AppHandle, width: f64, height: f64) {
    let Some(w) = popover(app) else { return };
    let _ = w.set_size(LogicalSize::new(width.clamp(200.0, 600.0), height.clamp(100.0, 900.0)));
    place(app, &w);
}

pub fn more_menu<R: Runtime>(w: &WebviewWindow<R>) -> tauri::Result<()> {
    let review = MenuItem::with_id(w, "review", "专注回顾", true, None::<&str>)?;
    let settings = MenuItem::with_id(w, "settings", "设置…", true, Some("CmdOrCtrl+,"))?;
    let quit = MenuItem::with_id(w, "quit", "退出 PomoSupervisor", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(w, &[&review, &settings, &PredefinedMenuItem::separator(w)?, &quit])?;
    w.popup_menu(&menu)
}

pub fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => show_popover(app, "normal"),
        "review" | "settings" => open(app, id),
        "quit" => app.exit(0),
        _ => {}
    }
}

/// Open (or bring forward) the Review or Settings window.
pub fn open(app: &AppHandle, label: &str) {
    if let Some(p) = popover(app) {
        let _ = p.hide();
    }
    set_dock(app, true);
    #[cfg(target_os = "macos")]
    let _ = app.show();
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.emit_to(label, "window://shown", ());
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let (title, h) = if label == "review" { ("专注回顾", 640.0) } else { ("设置", 560.0) };
    let b = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(800.0, h)
        .min_inner_size(720.0, 480.0)
        .center()
        .focused(true);
    #[cfg(target_os = "macos")]
    let b = b
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(14.0, 22.0));
    if let Err(e) = b.build() {
        eprintln!("open {label}: {e}");
    }
}

/// A menu-bar app: show in the Dock and app switcher only while a Review or Settings window is open.
pub fn set_dock(app: &AppHandle, visible: bool) {
    #[cfg(target_os = "macos")]
    {
        let policy = if visible { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory };
        let _ = app.set_activation_policy(policy);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, visible);
}

pub fn on_window_closed(app: &AppHandle, closed: &str) {
    let others = ["review", "settings"].iter().any(|l| *l != closed && app.get_webview_window(l).is_some());
    if !others {
        set_dock(app, false);
    }
}
