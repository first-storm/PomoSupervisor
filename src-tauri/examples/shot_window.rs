//! Dev helper: save a PNG of the PomoSupervisor window. cargo run --example shot_window -- out.png
fn main() {
    let out = std::env::args().nth(1).unwrap_or("window.png".into());
    for w in xcap::Window::all().unwrap() {
        let (app, title) = (w.app_name().unwrap_or_default(), w.title().unwrap_or_default());
        if (title == "PomoSupervisor" || app.contains("pomo-supervisor")) && w.width().unwrap_or(0) > 200 {
            w.capture_image().unwrap().save(&out).unwrap();
            println!("saved {out} ({app} / {title:?})");
            return;
        }
    }
    println!("window not found");
}
