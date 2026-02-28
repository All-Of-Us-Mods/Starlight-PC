mod commands;
mod utils;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut log_builder = tauri_plugin_log::Builder::new();

    if cfg!(debug_assertions) {
        // Dev Mode: Terminal only, Info level
        log_builder = log_builder
            .targets([Target::new(TargetKind::Stdout)])
            .level(log::LevelFilter::Info);
    } else {
        // Prod Mode: File only, Error level
        log_builder = log_builder
            .targets([Target::new(TargetKind::LogDir {
                file_name: Some("logs".to_string()),
            })])
            .level(log::LevelFilter::Error);
    }

    tauri::Builder::default()
        .plugin(
            log_builder
                .level_for("hyper", log::LevelFilter::Warn)
                .level_for("reqwest", log::LevelFilter::Warn)
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Starlight")
                .inner_size(800.0, 600.0)
                // Allow the window to be resized even without decorations
                .resizable(true)
                .visible(false);

            let _window = win_builder.build().unwrap();

            log::info!("Starlight started");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bepinex::install_bepinex,
            commands::bepinex::download_bepinex_to_cache,
            commands::bepinex::clear_bepinex_cache,
            commands::bepinex::check_bepinex_cache_exists,
            commands::finder::detect_among_us,
            commands::finder::get_game_platform,
            commands::launch::launch_modded,
            commands::launch::launch_vanilla,
            commands::launch::get_xbox_app_id,
            commands::launch::prepare_xbox_launch,
            commands::launch::launch_xbox,
            commands::launch::cleanup_xbox_files,
            commands::mods::download_mod,
            commands::epic_commands::get_epic_auth_url,
            commands::epic_commands::epic_login_with_code,
            commands::epic_commands::epic_login_with_webview,
            commands::epic_commands::epic_try_restore_session,
            commands::epic_commands::epic_logout,
            commands::epic_commands::epic_is_logged_in,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
