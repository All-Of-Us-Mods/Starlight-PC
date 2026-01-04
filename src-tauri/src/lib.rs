mod commands;
mod utils;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_updater::UpdaterExt;

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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let mut win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Starlight")
                .inner_size(800.0, 600.0)
                // Allow the window to be resized even without decorations
                .resizable(true)
                .visible(false);

            // macOS: Use overlay to show native buttons over custom titlebar
            #[cfg(target_os = "macos")]
            {
                use tauri::TitleBarStyle;
                win_builder = win_builder
                    .title_bar_style(TitleBarStyle::Overlay)
                    .title("");
            }

            // Windows/Linux: Hide native bar to use our custom one
            #[cfg(not(target_os = "macos"))]
            {
                win_builder = win_builder.decorations(false);
            }

            let _window = win_builder.build().unwrap();

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                update(handle).await.unwrap();
            });

            log::info!("Starlight started");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::download::download_and_extract_zip,
            commands::download::download_bepinex_to_cache,
            commands::download::clear_bepinex_cache,
            commands::download::check_bepinex_cache_exists,
            commands::finder::detect_among_us,
            commands::finder::get_game_platform,
            commands::game::save_game_copy,
            commands::launch::launch_modded,
            commands::launch::launch_vanilla,
            commands::epic_commands::get_epic_auth_url,
            commands::epic_commands::epic_login_with_code,
            commands::epic_commands::epic_try_restore_session,
            commands::epic_commands::epic_logout,
            commands::epic_commands::epic_is_logged_in,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    log::info!("Checking for updates...");
    if let Some(update) = app.updater()?.check().await? {
        log::info!("Update available, downloading...");
        let mut downloaded = 0;

        // alternatively we could also call update.download() and update.install() separately
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    log::debug!("Downloaded {downloaded} from {content_length:?}");
                },
                || {
                    log::info!("Download finished");
                },
            )
            .await?;

        log::info!("Update installed, restarting application");
        app.restart();
    } else {
        log::info!("No updates available");
    }

    Ok(())
}
