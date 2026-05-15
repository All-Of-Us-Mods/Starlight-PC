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

    gpui_platform::application().run(|cx: &mut App| {
        theme::init(cx);
        settings::init(cx);
        backend::init(cx);

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        ui::text_input::register_keybindings(cx);
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

        cx.open_window(options, |_, cx| cx.new(|cx| workspace::Workspace::new(cx)))
            .unwrap();
    });
}
