//! The control protocol for a page that embeds PhotoCraft (cargo feature `page-control`).
//!
//! The desktop app serves `docs/control-protocol.md` over a local socket; a browser cannot listen on
//! one. With this feature the same requests reach the app through one function on the page:
//!
//! ```js
//! const reply = JSON.parse(await window.__photocraftControl(JSON.stringify({ method, params })));
//! ```
//!
//! Requests and replies are exactly the desktop protocol's, handled by the same `control::handle`
//! through [`PhotocraftApp::with_control`](photocraft_ui_egui::PhotocraftApp::with_control). Off by
//! default: a plain web build exposes nothing to the page.
//!
//! ## When a request is answered
//!
//! The app handles control requests at the start of a frame, and the browser decides when frames
//! happen: every 16 ms while the window is on screen, about once a second while it is not being
//! presented, so a page steering a background editor got one command a second. A request therefore
//! runs one egui pass of its own, at once, inside the call: the same pass a frame runs (the queue is
//! drained first, then the whole interface is laid out), only it is not painted. The reply is ready
//! when the call returns, whatever the display does.
//!
//! - A request that only reads ([`reads_only`]) asks for no frame at all. What its pass laid out is
//!   painted with the next frame something else asks for.
//! - Any other request asks for one frame afterwards, so the change is seen.
//! - A request that needs several passes (synthetic input such as `ui.click`, a job the caller waits
//!   for) gets them at up to [`PUMP_MS`] apart while it waits, again without the display.
//! - `ui.screenshot` reads pixels the display painted, so it waits for painted frames as before.
//! - Before the app has started, or if a pass cannot be run, the request falls back to waking the
//!   app and waiting for its next frame.
//!
//! One field is the page's own: `ui.set {themeTokens: {...}}` lays the embedding host's colours and
//! radii over the chosen preset (`photocraft_ui_egui::theme::host`). It is taken out of the request
//! here, the rest of `ui.set` goes to the app unchanged, and the reply's `result` becomes
//! `{"themeTokens": {"applied": [...], "unknown": [...], "invalid": [...]}}`.

use serde_json::Value;

#[cfg(target_arch = "wasm32")]
mod bridge;
#[cfg(target_arch = "wasm32")]
pub use bridge::{attach, runner};

/// The least time between two passes run for a request that is still waiting, in milliseconds: a
/// display frame, so a waiting request costs no more passes than a window on screen would.
pub const PUMP_MS: f64 = 16.0;

/// `engine.execute` commands that only read.
const READ_COMMANDS: &[&str] = &["document.inspect", "session.inspect", "document.pixel"];

/// Whether a request only reads state. Answering one changes nothing on screen, so no frame is
/// asked for on its behalf. Anything not listed here is taken to change something.
pub fn reads_only(method: &str, params: &Value) -> bool {
    match method {
        "ui.inspect" | "ui.menu.list" | "engine.commands" => true,
        "engine.execute" => params.get("command").and_then(Value::as_str).is_some_and(|command| READ_COMMANDS.contains(&command)),
        _ => false,
    }
}

/// Whether a request that is still waiting gets another pass now: the app asked for a repaint and
/// the last pass was at least [`PUMP_MS`] ago.
pub fn pump_due(wants_repaint: bool, now_ms: f64, last_pass_ms: f64) -> bool {
    wants_repaint && now_ms - last_pass_ms >= PUMP_MS
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn reading_requests_ask_for_no_frame() {
        for (method, params) in [
            ("ui.inspect", json!({})),
            ("ui.inspect", Value::Null),
            ("ui.menu.list", json!({})),
            ("engine.commands", json!({})),
            ("engine.execute", json!({"command": "document.inspect"})),
            ("engine.execute", json!({"command": "session.inspect", "params": {}})),
            ("engine.execute", json!({"command": "document.pixel", "params": {"x": 1, "y": 1}})),
        ] {
            assert!(reads_only(method, &params), "{method} {params}");
        }
    }

    #[test]
    fn everything_else_asks_for_a_frame() {
        for (method, params) in [
            ("ui.set", json!({"tool": "brush"})),
            ("ui.set", json!({"themeTokens": {"accent": "#ff0000"}})),
            ("ui.menu.invoke", json!({"id": "file.new"})),
            ("ui.click", json!({"x": 1, "y": 1})),
            ("ui.screenshot", json!({})),
            ("engine.execute", json!({"command": "file.new", "params": {"width": 8, "height": 8}})),
            ("engine.execute", json!({"command": "edit.undo"})),
            ("engine.execute", json!({})),
            ("engine.execute", json!({"command": 7})),
            ("document.inspect", json!({})),
            ("nope", json!({})),
        ] {
            assert!(!reads_only(method, &params), "{method} {params}");
        }
    }

    #[test]
    fn a_waiting_request_is_pumped_no_faster_than_the_display() {
        assert!(pump_due(true, 1016.0, 1000.0));
        assert!(pump_due(true, 5000.0, 1000.0));
        assert!(!pump_due(true, 1015.9, 1000.0), "not twice within one display frame");
        assert!(!pump_due(false, 5000.0, 1000.0), "not when the app is idle");
    }

    /// The bridge leans on these in the web runner; a pull that renames them must fail here, not
    /// in the browser.
    #[test]
    fn the_web_shell_hands_its_runner_to_the_bridge() {
        let web = include_str!("web.rs");
        assert!(web.contains("crate::page_control::runner(&runner);"), "web.rs no longer gives the bridge the runner: requests wait for frames again");
        assert!(web.contains("crate::page_control::attach(app, &cc.egui_ctx)"), "web.rs no longer attaches the bridge");
    }
}
