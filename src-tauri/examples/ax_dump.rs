//! What the Accessibility API reads from the frontmost app.
//! Run: cargo run --example ax_dump [pid]   (no pid: switch to the app within 3 seconds)
fn main() {
    if !pomo_supervisor_lib::ax::trusted() {
        println!("not trusted: grant Accessibility to this terminal, then run again");
        pomo_supervisor_lib::ax::request();
        return;
    }
    // A pid argument reads that app (even in the background); none reads the frontmost one after 3 s.
    // A second argument is the app's name (Firefox gets asked differently).
    let name = std::env::args().nth(2).unwrap_or_default();
    let pid = match std::env::args().nth(1).and_then(|a| a.parse::<i32>().ok()) {
        Some(pid) => pid,
        None => {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let w = active_win_pos_rs::get_active_window().expect("active window");
            println!("app={:?} title={:?}", w.app_name, w.title);
            w.process_id as i32
        }
    };
    println!("pid={pid}");
    if let Ok(d) = std::env::var("AX_TREE") {
        println!("{}", pomo_supervisor_lib::ax::debug_tree(pid, d.parse().unwrap_or(8)));
    }
    let t = std::time::Instant::now();
    let info = pomo_supervisor_lib::ax::read(pid, &name);
    println!("took {:?}", t.elapsed());
    match info {
        Some(i) => {
            println!("focused: {:?}\nurl: {:?}\n--- text ---\n{}", i.focused, i.url, i.text.unwrap_or_default());
        }
        None => println!("none"),
    }
}
