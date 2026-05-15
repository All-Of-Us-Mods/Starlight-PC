// Epic webview login flow.
//
// The original Tauri implementation popped a WebviewWindow pointed at Epic's
// OAuth page and scraped the authorization code via injected JS. GPUI has no
// in-process webview, so this needs a new approach (system browser + custom
// URL scheme handler, or an external `wry` window). Stubbed for now so the
// rest of the crate compiles.

pub fn open_epic_login_window() -> Result<(), String> {
    Err("Epic webview login is not yet implemented on the GPUI port".to_string())
}
