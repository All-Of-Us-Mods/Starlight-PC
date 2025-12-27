mod commands;
use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .setup(|app| {
            let mut win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Starlight")
                .inner_size(800.0, 600.0)
                // Allow the window to be resized even without decorations
                .resizable(true);

            // macOS: Keep native buttons, make bar transparent
            #[cfg(target_os = "macos")]
            {
                win_builder = win_builder
                    .title_bar_style(TitleBarStyle::Transparent)
                    .fullsize_content_view(true);
            }

            // Windows/Linux: Hide native bar to use our custom one
            #[cfg(not(target_os = "macos"))]
            {
                win_builder = win_builder.decorations(false);
            }

            let _window = win_builder.build().unwrap();

            // macOS specific background styling
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::{NSColor, NSWindow};
                use cocoa::base::{id, nil};

                let ns_window = window.ns_window().unwrap() as id;
                unsafe {
                    let bg_color = NSColor::colorWithRed_green_blue_alpha_(
                        nil,
                        18.0 / 255.0, // Match your bg-card color
                        18.0 / 255.0,
                        18.0 / 255.0,
                        1.0,
                    );
                    ns_window.setBackgroundColor_(bg_color);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::profiles_backend::set_dll_directory,
            commands::profiles_backend::check_among_us_running
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
