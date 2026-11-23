//! Native smoke test: active window, idle time, screenshot + pHash.
//! Run: cargo run --example smoke
fn main() {
    match active_win_pos_rs::get_active_window() {
        Ok(w) => println!("active: app={:?} title={:?} path={:?}", w.app_name, w.title, w.process_path),
        Err(_) => println!("active: ERROR"),
    }
    match user_idle::UserIdle::get_time() {
        Ok(t) => println!("idle: {}s", t.as_seconds()),
        Err(e) => println!("idle: ERROR {e}"),
    }
    match xcap::Monitor::all() {
        Ok(ms) => {
            for m in ms.iter().take(1) {
                match m.capture_image() {
                    Ok(img) => {
                        let h = image_hasher::HasherConfig::new().to_hasher().hash_image(&img);
                        println!("capture: {}x{} phash={}", img.width(), img.height(), h.to_base64());
                    }
                    Err(e) => println!("capture: ERROR {e}"),
                }
            }
        }
        Err(e) => println!("monitors: ERROR {e}"),
    }
}
