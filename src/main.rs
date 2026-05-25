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

    let http = std::sync::Arc::new(
        reqwest_client::ReqwestClient::user_agent("Starlight").expect("http client"),
    );

    gpui_platform::application()
        .with_assets(ui::icon::EmbeddedAssets)
        .with_http_client(http)
        .run(|cx: &mut App| {
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
                titlebar: Some(TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(12.0), px(12.0))),
                }),
                ..Default::default()
            };

            cx.open_window(options, |window, cx| {
                let workspace = cx.new(|cx| workspace::Workspace::new(window, cx));
                let workspace_view: AnyView = workspace.into();
                cx.new(|cx| gpui_component::Root::new(workspace_view, window, cx))
            })
            .unwrap();
        });
}
