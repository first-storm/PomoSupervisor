use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Sample {
    pub app: String,
    /// The `.app` bundle's name from the process path, e.g. "Visual Studio Code".
    pub bundle_name: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub idle_secs: u64,
    pub pid: u64,
    /// Centre of the active window, to pick the display to capture.
    pub cx: i32,
    pub cy: i32,
}

pub fn idle_secs() -> u64 {
    user_idle::UserIdle::get_time().map(|t| t.as_seconds()).unwrap_or(0)
}

fn bundle_name(path: &std::path::Path) -> Option<String> {
    path.ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().into_owned())
}

/// AppleScript that reads the frontmost tab's URL, for browsers that support it.
fn url_script(app: &str) -> Option<String> {
    match app {
        "Safari" | "Safari Technology Preview" => Some(format!("tell application \"{app}\" to get URL of front document")),
        "Google Chrome" | "Google Chrome Canary" | "Chromium" | "Arc" | "Microsoft Edge" | "Brave Browser" | "Vivaldi" | "Opera" => {
            Some(format!("tell application \"{app}\" to get URL of active tab of front window"))
        }
        _ => None,
    }
}

async fn browser_url(app: &str) -> Option<String> {
    let script = url_script(app)?;
    let child = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(Duration::from_secs(3), child).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!url.is_empty() && url != "missing value").then_some(url)
}

pub async fn sample() -> Option<Sample> {
    let (win, idle) = tokio::task::spawn_blocking(|| (active_win_pos_rs::get_active_window().ok(), idle_secs()))
        .await
        .ok()?;
    let win = win?;
    let bundle = bundle_name(&win.process_path);
    let url_app = bundle.clone().unwrap_or_else(|| win.app_name.clone());
    let url = browser_url(&url_app).await;
    let p = &win.position;
    let (cx, cy) = ((p.x + p.width / 2.0) as i32, (p.y + p.height / 2.0) as i32);
    Some(Sample { app: win.app_name, bundle_name: bundle, title: win.title, url, idle_secs: idle, pid: win.process_id, cx, cy })
}
