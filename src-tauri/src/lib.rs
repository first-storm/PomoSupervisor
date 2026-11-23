mod app;
pub mod ax;
mod commands;
mod decision;
mod llm;
mod pipeline;
mod rules;
mod sampler;
mod settings;
mod smoother;
mod store;
mod timer;
mod vision;
mod windows;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // A menu-bar app: no Dock icon until a Review or Settings window opens.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data = app.path().app_data_dir()?;
            let state = app::AppState::new(data.join("settings.json"), data.join("pomo.db"))?;
            app.manage(Arc::new(state));

            let show = MenuItem::with_id(app, "show", "打开 PomoSupervisor", true, None::<&str>)?;
            let review = MenuItem::with_id(app, "review", "上一轮回顾", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show, &review, &settings, &sep, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(tauri::include_image!("icons/tray.png"))
                .icon_as_template(true)
                .tooltip("PomoSupervisor")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        windows::toggle_popover(tray.app_handle());
                    }
                })
                .build(app)?;

            app::spawn_loop(app.handle().clone());

            // Dev aid: POMO_OPEN=main|review|settings opens that window at launch (debug builds only).
            #[cfg(debug_assertions)]
            if let Ok(which) = std::env::var("POMO_OPEN") {
                let h = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
                    match which.as_str() {
                        "main" => windows::show_popover(&h, "normal"),
                        "nudge" => {
                            let st = h.state::<Arc<app::AppState>>();
                            st.start_focus("调试：分心提醒".into(), Some(1), vec![]);
                            app::emit_tick(&h, &st);
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            windows::show_popover(&h, "nudge");
                        }
                        other => windows::open(&h, other),
                    }
                });
            }
            Ok(())
        })
        // Menu events from the tray menu and the popover's "more" menu.
        .on_menu_event(|app, e| windows::on_menu(app, e.id.as_ref()))
        .on_window_event(|window, event| match (window.label(), event) {
            // The popover never closes; it hides when it loses focus.
            ("main", tauri::WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            ("main", tauri::WindowEvent::Focused(false)) => windows::on_popover_blur(window),
            // Settings saves a moment after each edit: let it flush before the window goes.
            ("settings", tauri::WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.emit_to("settings", "window://closing", ());
                let w = window.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    let _ = w.destroy();
                });
            }
            (label, tauri::WindowEvent::Destroyed) => windows::on_window_closed(window.app_handle(), label),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::last_verdict,
            commands::start_focus,
            commands::start_break,
            commands::pause,
            commands::resume,
            commands::stop,
            commands::rest,
            commands::allow_for_round,
            commands::mark_as_work,
            commands::set_auto_resume,
            commands::recent_verdicts,
            commands::get_review,
            commands::today_completed,
            commands::open_window,
            commands::close_window,
            commands::popover_fit,
            commands::popover_menu,
            commands::get_settings,
            commands::save_settings,
            commands::model_status,
            commands::set_api_key,
            commands::test_provider,
            commands::get_stats,
            commands::check_permissions,
            commands::request_screen_recording,
            commands::request_accessibility,
            commands::open_privacy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
