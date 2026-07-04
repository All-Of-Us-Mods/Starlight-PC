use gpui::*;

mod app;
mod backend;
mod settings;
mod theme;
mod ui;
mod views;
mod workspace;

actions!(starlight, [Quit]);

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("starlight starting");

    #[cfg(windows)]
    backend::services::update_service::cleanup_leftover_old_exe();

    #[cfg(windows)]
    if let Err(e) = backend::services::profile_shortcut_service::register_deep_link_scheme() {
        log::warn!("failed to register starlight:// url scheme: {e}");
    }

    // Desktop shortcuts open the app as `starlight.exe starlight://profile/{id}`
    // (via the registered url scheme) — launch that profile once the app is up.
    let deep_link_profile = std::env::args()
        .skip(1)
        .find_map(|arg| backend::services::profile_shortcut_service::parse_profile_deep_link(&arg));

    let http = std::sync::Arc::new(
        reqwest_client::ReqwestClient::user_agent("Starlight").expect("http client"),
    );

    gpui_platform::application()
        .with_assets(ui::icon::EmbeddedAssets)
        .with_http_client(http)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
            ui::log_language::register();
            theme::init(cx);
            settings::init(cx);
            backend::init(cx);

            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.set_menus(vec![Menu {
                name: "Starlight".into(),
                disabled: false,
                items: vec![MenuItem::action("Quit Starlight", Quit)],
            }]);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1024.), px(768.)),
                    cx,
                ))),
                titlebar: Some(gpui_component::TitleBar::title_bar_options()),
                // Keep the window from shrinking small enough that the nav +
                // settings sidebars crowd the content off-screen.
                window_min_size: Some(size(px(820.0), px(600.0))),
                app_id: Some("starlight".into()),
                ..Default::default()
            };

            cx.open_window(options, |window, cx| {
                let workspace = cx.new(|cx| workspace::Workspace::new(window, cx));
                let workspace_view: AnyView = workspace.into();
                cx.new(|cx| gpui_component::Root::new(workspace_view, window, cx))
            })
            .unwrap();

            if let Some(profile_id) = deep_link_profile {
                cx.background_executor()
                    .spawn(async move {
                        use backend::services::{launch_service, profile_service};
                        let result = profile_service::get_profile_by_id(&profile_id)
                            .and_then(|profile| {
                                profile.ok_or_else(|| {
                                    backend::error::AppError::validation(format!(
                                        "Profile '{profile_id}' not found"
                                    ))
                                })
                            })
                            .and_then(launch_service::launch_modded_for_profile);
                        if let Err(e) = result {
                            log::warn!("deep-link profile launch failed: {e}");
                        }
                    })
                    .detach();
            }
        });
}
