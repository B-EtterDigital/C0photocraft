//! The `c0.*` requests of the C0VIBE host's sessions: named groups of documents shown as tabs.
//!
//! This file is not upstream PhotoCraft's: it belongs to the C0VIBE host shell (see
//! `c0/photocraft-host/README.md`). `c0_generative::handle` hands it every `c0.*` method it does not
//! know. The ids are the engine's `DocState::uid` (`photocraft_engine`'s `c0_sessions`): the same from
//! the moment a document opens until it closes, never reused in a run, listed as `id` in
//! `ui.inspect().session.documents[]`.
//!
//! | Method | Params | Result |
//! |---|---|---|
//! | `c0.activate` | `{id}` | `{id, document}` |
//! | `c0.close` | `{id, discard?}` | `{id, closed, prompt}` |
//! | `c0.tabs` | `{visible: [id] \| null}` (no `visible`: only reports) | `{visible, active}` |
//! | `c0.documentBytes` | `{id?, format?: "pcraft"}` | `{name, format, bytes, revision}` |
//!
//! `bytes` is base64 (standard alphabet, no `data:` prefix). `c0.capture`, `c0.selectionMask` and
//! `c0.place` take the same optional `id` to act on a document other than the active one.

use base64::Engine as _;
use serde_json::{Value, json};

use crate::PhotocraftApp;
use crate::tab_strip::{self, TabFit};

/// The methods of this module, for the host contract test.
pub const METHODS: [&str; 4] = ["c0.activate", "c0.close", "c0.tabs", "c0.documentBytes"];

/// The most bytes of a document a reply carries: its base64 is 96 MiB, the largest `c0.*` message.
const MAX_BYTES: usize = 72 * 1024 * 1024;

/// Answers a `c0.*` request this module knows; an error for any other.
pub fn handle(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "c0.activate" => activate(app, params),
        "c0.close" => close(app, ctx, params),
        "c0.tabs" => tabs(app, params),
        "c0.documentBytes" => document_bytes(app, params),
        other => Err(format!("unknown method `{other}`")),
    }
}

/// [`tab_strip::fit`] for the document tab strips: only the tabs the host's filter shows are laid
/// out (opening files, which come after the documents in `natural`, always are). The indices of the
/// result are `natural`'s.
pub fn fit(app: &PhotocraftApp, natural: &[f32], selected: usize, avail: f32, min_w: f32, chevron_w: f32) -> TabFit {
    if app.session.visible().is_none() {
        return tab_strip::fit(natural, selected, avail, min_w, chevron_w);
    }
    let documents = app.session.documents().len();
    let keep: Vec<usize> = (0..natural.len()).filter(|&i| i >= documents || app.session.tab_visible(i)).collect();
    let widths: Vec<f32> = keep.iter().map(|&i| natural[i]).collect();
    let selected = keep.iter().position(|&i| i == selected).unwrap_or(0);
    let mut f = tab_strip::fit(&widths, selected, avail, min_w, chevron_w);
    for (i, _) in &mut f.shown {
        *i = keep[*i];
    }
    for i in &mut f.overflow {
        *i = keep[*i];
    }
    f
}

/// The id of the document at `index`.
pub fn id_of(app: &PhotocraftApp, index: usize) -> Option<u64> {
    app.session.documents().get(index).map(|d| d.uid)
}

/// The document a request is about: the one its `id` names, else the active one.
pub fn target_index(app: &PhotocraftApp, p: &Value) -> Result<usize, String> {
    match p.get("id") {
        None | Some(Value::Null) => app.session.active_index().ok_or_else(|| "no document open".to_owned()),
        Some(v) => {
            let id = whole_id(v, "id")?;
            app.session.index_of_uid(id).ok_or_else(|| format!("no document with id {id}"))
        }
    }
}

fn whole_id(v: &Value, what: &str) -> Result<u64, String> {
    v.as_u64().ok_or_else(|| format!("`{what}` must be a whole document id"))
}

/// The id a request needs (`id` is required here).
fn required_index(app: &PhotocraftApp, p: &Value) -> Result<(u64, usize), String> {
    if p.get("id").is_none_or(Value::is_null) {
        return Err("`id` is required".into());
    }
    let index = target_index(app, p)?;
    Ok((app.session.documents()[index].uid, index))
}

fn active_id(app: &PhotocraftApp) -> Value {
    app.session.active_index().and_then(|i| id_of(app, i)).map_or(Value::Null, Value::from)
}

fn activate(app: &mut PhotocraftApp, p: &Value) -> Result<Value, String> {
    let (id, index) = required_index(app, p)?;
    if !app.session.tab_visible(index) {
        return Err(format!("document {id} is hidden by the tab filter"));
    }
    app.session.set_active(index);
    app.jobs.focus = None;
    Ok(json!({"id": id, "document": index}))
}

fn close(app: &mut PhotocraftApp, ctx: &egui::Context, p: &Value) -> Result<Value, String> {
    let (id, index) = required_index(app, p)?;
    let discard = match p.get("discard") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err("`discard` must be a boolean".into()),
    };
    let params = json!({"document": index});
    // Without `discard`: the app's own path, which asks about unsaved changes (the prompt is then
    // pending in the window, and the document is still open).
    if discard {
        crate::menus::invoke_unguarded(app, ctx, "file.close", params)?;
    } else {
        crate::menus::invoke(app, ctx, "file.close", params)?;
    }
    let closed = app.session.index_of_uid(id).is_none();
    Ok(json!({"id": id, "closed": closed, "prompt": !closed && app.discard.is_some()}))
}

fn tabs(app: &mut PhotocraftApp, p: &Value) -> Result<Value, String> {
    match p.get("visible") {
        None => {}
        Some(Value::Null) => app.session.set_visible(None)?,
        Some(Value::Array(list)) => {
            let ids = list.iter().map(|v| whole_id(v, "visible[]")).collect::<Result<Vec<_>, _>>()?;
            app.session.set_visible(Some(ids))?;
        }
        Some(_) => return Err("`visible` must be a list of document ids or null".into()),
    }
    Ok(json!({"visible": app.session.visible(), "active": active_id(app)}))
}

fn document_bytes(app: &PhotocraftApp, p: &Value) -> Result<Value, String> {
    match p.get("format") {
        None | Some(Value::Null) => {}
        Some(Value::String(f)) if f == photocraft_format::EXTENSION => {}
        Some(_) => return Err(format!("`format` must be \"{}\"", photocraft_format::EXTENSION)),
    }
    let st = &app.session.documents()[target_index(app, p)?];
    // As File > Save writes it, from the document as it is: nothing in it is touched.
    let name = format!("document.{}", photocraft_format::EXTENSION);
    let exported = photocraft_io::export(&st.doc, &name, &Default::default()).map_err(|e| e.to_string())?;
    if exported.bytes.len() > MAX_BYTES {
        return Err(format!("the document is {} MiB as a .pcraft: more than a reply carries ({} MiB)", exported.bytes.len() >> 20, MAX_BYTES >> 20));
    }
    Ok(json!({
        "name": st.doc.name,
        "format": photocraft_format::EXTENSION,
        "bytes": base64::engine::general_purpose::STANDARD.encode(&exported.bytes),
        "revision": st.revision,
    }))
}
