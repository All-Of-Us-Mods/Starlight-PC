pub mod commands;
pub mod error;
pub mod services;
pub mod state;

use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut log_builder = tauri_plugin_log::Builder::new();

    if cfg!(debug_assertions) {
        log_builder = log_builder
            .targets([Target::new(TargetKind::Stdout)])
            .level(log::LevelFilter::Info);
    } else {
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
                .resizable(true)
                .visible(false);

            #[cfg(target_os = "macos")]
            let win_builder = {
                use tauri::TitleBarStyle;
                win_builder
                    .title_bar_style(TitleBarStyle::Overlay)
                    .title("")
            };

            #[cfg(not(target_os = "macos"))]
            let win_builder = win_builder.decorations(false);

            let _window = win_builder.build().unwrap();
            log::info!("Starlight started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::core::core_get_settings,
            commands::core::core_update_settings,
            commands::core::core_get_bepinex_cache_path,
            commands::core::core_get_app_data_dir,
            commands::core::core_auto_detect_bepinex_architecture,
            commands::game::game_launch_modded,
            commands::game::game_launch_vanilla,
            commands::game::game_xbox_get_app_id,
            commands::game::game_xbox_prepare_launch,
            commands::game::game_xbox_launch,
            commands::game::game_xbox_cleanup,
            commands::platform::platform_detect_among_us,
            commands::platform::platform_detect_game_store,
            commands::mods::modding_bepinex_install,
            commands::mods::modding_bepinex_cache_download,
            commands::mods::modding_bepinex_cache_clear,
            commands::mods::modding_bepinex_cache_exists,
            commands::mods::modding_mod_download,
            commands::profiles::profiles_export_zip,
            commands::profiles::profiles_import_zip,
            commands::epic::epic_auth_url,
            commands::epic::epic_login_code,
            commands::epic::epic_login_webview,
            commands::epic::epic_session_restore,
            commands::epic::epic_logout,
            commands::epic::epic_is_logged_in,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
