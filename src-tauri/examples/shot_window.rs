//! Dev helper: save a PNG of a PomoSupervisor window.
//!   cargo run --example shot_window -- out.png [popover|review|settings]
//! The popover is titled "PomoSupervisor", the others "本轮回顾" and "设置".
fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or("window.png".into());
    let want = match args.next().as_deref() {
        Some("review") => "本轮回顾",
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
    println!("window not found");
}
