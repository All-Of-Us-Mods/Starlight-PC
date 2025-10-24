#[cfg_attr(mobile, tauri::mobile_entry_point)]
mod commands;
mod utils;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(true)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    other => {
                        println!("menu item {other} not handled");
                    }
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::paths::get_among_us_paths,
            commands::init::init_app,
            commands::init::get_among_us_path_from_store,
            commands::init::update_among_us_path,
            commands::profiles::create_profile,
            commands::launch::launch_among_us
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
