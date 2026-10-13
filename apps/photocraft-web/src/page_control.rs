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
//! - A request of synthetic input (`ui.click`, `ui.move`, `ui.key`, `ui.type`, `ui.pointer`: see
//!   [`is_synthetic_input`]) is queued by the pass that reads it and played one step per pass, and
//!   its reply waits for the last step. Those passes are run inside the same call, back to back,
//!   up to [`inline_passes`] of them: a timer cannot be relied on to wake the page, because a page
//!   the browser throttles (hidden, or not presented) runs timers about once a minute, and a click
//!   that waited for the timer waited that long. Nothing needs a painted frame for the input to
//!   reach the app; eframe's `raw_input_hook` runs in every pass, painted or not.
//! - A job the caller waits for gets its passes at up to [`PUMP_MS`] apart while it waits, again
//!   without the display. Whatever input is still unplayed after the inline passes is finished the
//!   same way.
//! - `ui.screenshot` reads pixels the display painted, so it waits for painted frames as before.
//! - Before the app has started, or if a pass cannot be run, the request falls back to waking the
//!   app and waiting for its next frame.
//!
//! ## The host's `c0.*` requests
//!
//! `c0.capture`, `c0.selectionMask`, `c0.place`, `c0.openFile` and `c0.cancelFileDialog` carry or
//! return images as base64 and are answered by the app like any other request
//! (`photocraft_ui_egui::c0_generative`, which a native test runs). Because they carry files, a
//! request of this family may be up to [`C0_MAX_REQUEST_BYTES`] long; any other request keeps the
//! desktop socket's 1 MiB ([`request_limit`]). `c0.capture`, `c0.selectionMask` and
//! `c0.documentBytes` only read. The sessions requests (`c0.activate`, `c0.close`, `c0.tabs`,
//! `c0.documentBytes`; `photocraft_ui_egui::c0_sessions`) are the same family; the tab filter and
//! activation change what is on screen, so those ask for a frame.
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

/// The largest ordinary request, as on the desktop socket.
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
/// The largest `c0.*` request: it carries a file or an image as base64 (a PSD of 48 MiB is 64 MiB).
pub const C0_MAX_REQUEST_BYTES: usize = 96 * 1024 * 1024;

/// The most bytes a request of `method` may be.
pub fn request_limit(method: &str) -> usize {
    if method.starts_with("c0.") { C0_MAX_REQUEST_BYTES } else { MAX_REQUEST_BYTES }
}

/// `engine.execute` commands that only read.
const READ_COMMANDS: &[&str] = &["document.inspect", "session.inspect", "document.pixel"];

/// Whether a request only reads state. Answering one changes nothing on screen, so no frame is
/// asked for on its behalf. Anything not listed here is taken to change something.
pub fn reads_only(method: &str, params: &Value) -> bool {
    match method {
        "ui.inspect" | "ui.menu.list" | "engine.commands" | "c0.capture" | "c0.selectionMask" | "c0.documentBytes" => true,
        "engine.execute" => params.get("command").and_then(Value::as_str).is_some_and(|command| READ_COMMANDS.contains(&command)),
        _ => false,
    }
}

/// The control requests that queue synthetic input (`photocraft_ui_egui::control`): the app plays
/// them one step per pass and answers after the last, so their reply is never ready after the first
/// pass. `ui.pointer` is answered by the pass that reads it, but it is in the family: it feeds the
/// same tools and a caller treats the five alike.
pub const SYNTHETIC_INPUT: &[&str] = &["ui.click", "ui.move", "ui.key", "ui.type", "ui.pointer"];

/// The most passes run inside one call for a synthetic input request; what is left is played while
/// the request waits ([`pump_due`]), so a `count` of 256 does not stall the page.
pub const MAX_INLINE_PASSES: usize = 32;

/// Whether `method` is a synthetic input request.
pub fn is_synthetic_input(method: &str) -> bool {
    SYNTHETIC_INPUT.contains(&method)
}

/// The passes to run inside the call after the one that read the request: one per step the app
/// plays (a click is one step, so `count` clicks are `count`), one more to answer, at most
/// [`MAX_INLINE_PASSES`]. None for any other request.
pub fn inline_passes(method: &str, params: &Value) -> usize {
    if !is_synthetic_input(method) {
        return 0;
    }
    let steps = if method == "ui.click" { params.get("count").and_then(Value::as_u64).unwrap_or(1) } else { 1 };
    (steps.saturating_add(1) as usize).min(MAX_INLINE_PASSES)
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
            ("c0.capture", json!({"scope": "document"})),
            ("c0.selectionMask", json!({"rect": {"x": 0, "y": 0, "width": 4, "height": 4}})),
            ("c0.documentBytes", json!({"id": 1})),
        ] {
            assert!(reads_only(method, &params), "{method} {params}");
        }
    }

    #[test]
    fn c0_requests_may_carry_files_and_others_may_not() {
        assert_eq!(request_limit("c0.openFile"), C0_MAX_REQUEST_BYTES);
        assert_eq!(request_limit("c0.place"), C0_MAX_REQUEST_BYTES);
        for method in ["ui.set", "engine.execute", "c0", "xc0.place", ""] {
            assert_eq!(request_limit(method), MAX_REQUEST_BYTES, "{method}");
        }
        assert!(C0_MAX_REQUEST_BYTES > 64 * 1024 * 1024, "room for a 48 MiB file as base64");
    }

    /// What changes the document or the dialog state asks for a frame.
    #[test]
    fn c0_requests_that_change_something_ask_for_a_frame() {
        for method in ["c0.place", "c0.openFile", "c0.cancelFileDialog", "c0.activate", "c0.close", "c0.tabs"] {
            assert!(!reads_only(method, &json!({})), "{method}");
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

    /// A synthetic input request is not through after its first pass, so the bridge must give it
    /// passes inside the call. Every method that queues input is in the rule; nothing else is.
    #[test]
    fn every_synthetic_input_method_gets_inline_passes() {
        for (method, params) in [
            ("ui.move", json!({"x": 10, "y": 10})),
            ("ui.click", json!({"x": 1, "y": 1})),
            ("ui.key", json!({"key": "A"})),
            ("ui.type", json!({"text": "x"})),
            ("ui.pointer", json!({"events": []})),
        ] {
            assert!(is_synthetic_input(method), "{method}");
            assert!(inline_passes(method, &params) >= 2, "{method}: one pass to queue, one to play, and one to answer");
            assert!(!reads_only(method, &params), "{method} changes the screen: it asks for a frame");
        }
        for method in ["ui.inspect", "ui.set", "ui.menu.invoke", "ui.screenshot", "engine.execute", "c0.place", "ui.clicks", ""] {
            assert!(!is_synthetic_input(method), "{method}");
            assert_eq!(inline_passes(method, &json!({"count": 9})), 0, "{method}");
        }
    }

    #[test]
    fn a_click_gets_a_pass_for_each_of_its_clicks_up_to_the_cap() {
        assert_eq!(inline_passes("ui.click", &json!({"x": 1, "y": 1})), 2);
        assert_eq!(inline_passes("ui.click", &json!({"x": 1, "y": 1, "count": 2})), 3);
        assert_eq!(inline_passes("ui.click", &json!({"count": 3})), 4);
        assert_eq!(inline_passes("ui.click", &json!({"count": 256})), MAX_INLINE_PASSES);
        assert_eq!(inline_passes("ui.click", &json!({"count": u64::MAX})), MAX_INLINE_PASSES);
        assert_eq!(inline_passes("ui.click", &json!({"count": "x"})), 2, "a bad count is the app's to refuse");
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
