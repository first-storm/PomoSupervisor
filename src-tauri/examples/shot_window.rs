//! Dev helper: save a PNG of a PomoSupervisor window.
//!   cargo run --example shot_window -- out.png [popover|review|settings]
//! The popover is titled "PomoSupervisor"; the others are matched by their zh-Hans titles,
//! "专注回顾" and "设置", so run the app with POMO_LANG=zh-Hans.
fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or("window.png".into());
    let want = match args.next().as_deref() {
        Some("review") => "专注回顾",
        Some("settings") => "设置",
        _ => "PomoSupervisor",
    };
    for w in xcap::Window::all().unwrap() {
        let (app, title) = (w.app_name().unwrap_or_default(), w.title().unwrap_or_default());
        let ours = app.contains("pomo-supervisor") || app == "PomoSupervisor";
        if ours && title == want && w.width().unwrap_or(0) > 200 {
            w.capture_image().unwrap().save(&out).unwrap();
            println!("saved {out} ({app} / {title:?})");
            return;
        }
    }
    println!("window not found; windows of ours on screen:");
    for w in xcap::Window::all().unwrap() {
        let (app, title) = (w.app_name().unwrap_or_default(), w.title().unwrap_or_default());
        if app.contains("pomo-supervisor") || app == "PomoSupervisor" {
            println!("  {app} / {title:?} ({}x{})", w.width().unwrap_or(0), w.height().unwrap_or(0));
        }
    }
}
