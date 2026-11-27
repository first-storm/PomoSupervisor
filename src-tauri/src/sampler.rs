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

/// Tools whose whole job is keeping the Mac awake: they say nothing about anyone watching.
const KEEP_AWAKE_TOOLS: [&str; 5] = ["caffeinate", "Amphetamine", "Lungo", "KeepingYouAwake", "Caffeine"];

/// `pmset -g assertions` output: is some app (a playing video, a call) keeping the display awake?
/// Read from the per-process lines, `pid 123(Name): [..] 00:00:01 PreventUserIdleDisplaySleep named: ..`.
fn display_held(pmset: &str) -> bool {
    pmset.lines().map(str::trim_start).filter(|l| l.starts_with("pid ")).any(|l| {
        let owner = l.split_once('(').and_then(|(_, rest)| rest.split_once("):")).map(|(name, _)| name).unwrap_or("");
        let holds = l.contains(" PreventUserIdleDisplaySleep ") || l.contains(" NoDisplaySleepAssertion ");
        holds && !KEEP_AWAKE_TOOLS.contains(&owner)
    })
}

/// No input for a while, yet a video or a call holds the display on: the person is
/// watching, not gone. Only asked once the idle threshold has passed.
pub async fn display_held_awake() -> bool {
    let child = tokio::process::Command::new("pmset").args(["-g", "assertions"]).kill_on_drop(true).output();
    match tokio::time::timeout(Duration::from_secs(2), child).await {
        Ok(Ok(out)) => display_held(&String::from_utf8_lossy(&out.stdout)),
        _ => false,
    }
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
    let child = tokio::process::Command::new("osascript").arg("-e").arg(script).kill_on_drop(true).output();
    let out = tokio::time::timeout(Duration::from_secs(3), child).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!url.is_empty() && url != "missing value").then_some(url)
}

pub async fn sample() -> Option<Sample> {
    let (win, idle) = tokio::task::spawn_blocking(|| (active_win_pos_rs::get_active_window().ok(), idle_secs())).await.ok()?;
    let win = win?;
    let bundle = bundle_name(&win.process_path);
    let url_app = bundle.clone().unwrap_or_else(|| win.app_name.clone());
    let url = browser_url(&url_app).await;
    let p = &win.position;
    let (cx, cy) = ((p.x + p.width / 2.0) as i32, (p.y + p.height / 2.0) as i32);
    Some(Sample { app: win.app_name, bundle_name: bundle, title: win.title, url, idle_secs: idle, pid: win.process_id, cx, cy })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_display_assertion() {
        let head = "Assertion status system-wide:\n   PreventUserIdleDisplaySleep    1\nListed by owning process:\n";
        let video = "   pid 812(Google Chrome): [0x0001] 00:03:10 PreventUserIdleDisplaySleep named: \"Video Wake Lock\"  \n";
        let tool = "   pid 8269(caffeinate): [0x0002] 00:00:01 PreventUserIdleDisplaySleep named: \"caffeinate command-line tool\"  \n";
        let audio = "   pid 559(coreaudiod): [0x0003] 00:17:50 PreventUserIdleSystemSleep named: \"audio\"  \n";
        assert!(display_held(&format!("{head}{audio}{video}")));
        // A keep-awake tool is not somebody watching, and audio alone doesn't hold the display.
        assert!(!display_held(&format!("{head}{tool}{audio}")));
        assert!(display_held(&format!("{head}{tool}{video}")));
        assert!(!display_held(head));
    }
}
