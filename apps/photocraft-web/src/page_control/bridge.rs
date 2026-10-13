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
//! through [`PhotocraftApp::with_control`]. A request wakes the app (`request_repaint`), so it is
//! answered on the next frame even when nothing else is drawing. Off by default: a plain web build
//! exposes nothing to the page.
//!
//! One field is the page's own: `ui.set {themeTokens: {...}}` lays the embedding host's colours and
//! radii over the chosen preset (`photocraft_ui_egui::theme::host`). It is taken out of the request
//! here, the rest of `ui.set` goes to the app unchanged, and the reply's `result` becomes
//! `{"themeTokens": {"applied": [...], "unknown": [...], "invalid": [...]}}`.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use photocraft_ui_egui::control::ControlRequest;
use photocraft_ui_egui::theme::host;
use photocraft_ui_egui::PhotocraftApp;
use serde_json::{Value, json};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;

/// The function the page calls.
pub const PAGE_FUNCTION: &str = "__photocraftControl";
/// The largest request accepted, as on the desktop socket.
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
/// How often a waiting request checks for its reply.
const POLL_MS: i32 = 4;
/// The `ui.set` field that carries the host's palette.
const THEME_TOKENS: &str = "themeTokens";

/// A palette set by a `ui.set` request the app has not answered yet.
struct Palette {
    /// The set it replaced, put back if the app refuses the rest of the request.
    before: Option<Arc<Value>>,
    report: Value,
}

/// Wires the page function to `app`'s control channel.
pub fn attach(app: PhotocraftApp, ctx: &egui::Context) -> PhotocraftApp {
    let (tx, rx) = channel::<ControlRequest>();
    install(tx, ctx.clone());
    app.with_control(rx)
}

fn install(tx: Sender<ControlRequest>, ctx: egui::Context) {
    let Some(window) = web_sys::window() else {
        log::error!("page-control: no window");
        return;
    };
    let call = Closure::<dyn FnMut(JsValue) -> js_sys::Promise>::new(move |text: JsValue| {
        let reply = parse(&text).and_then(|(method, params)| {
            let (params, palette) = take_palette(&ctx, &method, params)?;
            Ok((submit(&tx, &ctx, method, params)?, palette))
        });
        match reply {
            Ok((rx, palette)) => wasm_bindgen_futures::future_to_promise(wait(rx, ctx.clone(), palette)),
            Err(error) => js_sys::Promise::resolve(&JsValue::from_str(&encode(&json!({"ok": false, "error": error})))),
        }
    });
    if let Err(error) = js_sys::Reflect::set(&window, &JsValue::from_str(PAGE_FUNCTION), call.as_ref().unchecked_ref()) {
        log::error!("page-control: could not install {PAGE_FUNCTION}: {error:?}");
        return;
    }
    // The page keeps the function for the life of the app.
    call.forget();
}

fn parse(text: &JsValue) -> Result<(String, Value), String> {
    let Some(text) = text.as_string() else { return Err("the request must be JSON text".into()) };
    if text.len() > MAX_REQUEST_BYTES {
        return Err("the request is larger than 1 MiB".into());
    }
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("the request is not JSON: {e}"))?;
    let Some(method) = value.get("method").and_then(Value::as_str) else { return Err("the request has no method".into()) };
    Ok((method.to_owned(), value.get("params").cloned().unwrap_or(Value::Null)))
}

/// Applies `ui.set`'s `themeTokens` at once (so the preset the same request selects is drawn with it)
/// and takes the field out: the app's `ui.set` refuses fields it does not know.
fn take_palette(ctx: &egui::Context, method: &str, mut params: Value) -> Result<(Value, Option<Palette>), String> {
    let tokens = if method == "ui.set" { params.as_object_mut().and_then(|fields| fields.remove(THEME_TOKENS)) } else { None };
    let Some(tokens) = tokens else { return Ok((params, None)) };
    let before = host::current(ctx);
    let report = host::set(ctx, &tokens)?.to_json();
    Ok((params, Some(Palette { before, report })))
}

/// Adds the palette's report to the app's reply, or undoes the palette when the app said no.
fn settle(ctx: &egui::Context, reply: &mut Value, palette: Palette) {
    if reply.get("ok").and_then(Value::as_bool) != Some(true) {
        host::put(ctx, palette.before);
        return;
    }
    if !reply["result"].is_object() {
        reply["result"] = json!({});
    }
    reply["result"][THEME_TOKENS] = palette.report;
}

fn submit(tx: &Sender<ControlRequest>, ctx: &egui::Context, method: String, params: Value) -> Result<Receiver<Value>, String> {
    let (reply, rx) = channel();
    // No deadline: `std::time::Instant` does not exist in the browser; the page enforces its own.
    tx.send(ControlRequest { method, params, reply, deadline: None }).map_err(|_| "PhotoCraft has stopped".to_owned())?;
    ctx.request_repaint();
    Ok(rx)
}

async fn wait(rx: Receiver<Value>, ctx: egui::Context, mut palette: Option<Palette>) -> Result<JsValue, JsValue> {
    loop {
        match rx.try_recv() {
            Ok(mut value) => {
                if let Some(palette) = palette.take() {
                    settle(&ctx, &mut value, palette);
                }
                return Ok(JsValue::from_str(&encode(&value)));
            }
            Err(TryRecvError::Disconnected) => {
                return Ok(JsValue::from_str(&encode(&json!({"ok": false, "error": "PhotoCraft dropped the request"}))));
            }
            Err(TryRecvError::Empty) => sleep(POLL_MS).await?,
        }
    }
}

async fn sleep(ms: i32) -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window() {
            // A failed timer resolves at once; the loop then asks again.
            if window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms).is_err() {
                let _ = resolve.call0(&JsValue::NULL);
            }
        } else {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.map(|_| ())
}

fn encode(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| r#"{"ok":false,"error":"the reply could not be encoded"}"#.to_owned())
}
