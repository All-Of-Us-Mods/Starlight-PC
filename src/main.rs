use gpui::*;

mod app;
mod settings;
mod theme;
mod workspace;
mod backend;

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        theme::init(cx);
        settings::init(cx);
        backend::init(cx);
        
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
        
        cx.open_window(options, |_, cx| {
            cx.new(|cx| workspace::Workspace::new(cx))
        }).unwrap();
    });
}
