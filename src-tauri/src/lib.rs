mod app;
mod commands;
mod llm;
mod pipeline;
mod rules;
mod sampler;
mod settings;
mod smoother;
mod store;
mod timer;
mod vision;

use std::sync::Arc;
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let state = app::AppState::new(data.join("settings.json"), data.join("pomo.db"))?;
            app.manage(Arc::new(state));

            let show = MenuItem::with_id(app, "show", "打开 PomoSupervisor", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(tauri::include_image!("icons/tray.png"))
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            app::spawn_loop(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the timer running in the menu bar.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::last_verdict,
            commands::start_focus,
            commands::start_break,
            commands::pause,
            commands::resume,
            commands::stop,
            commands::get_settings,
            commands::save_settings,
            commands::model_status,
            commands::set_api_key,
            commands::test_provider,
            commands::get_stats,
            commands::check_permissions,
            commands::request_screen_recording,
            commands::open_privacy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
