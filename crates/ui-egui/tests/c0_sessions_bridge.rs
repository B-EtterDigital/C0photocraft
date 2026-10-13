//! The `c0.*` requests of the C0VIBE host's sessions (`src/c0_sessions.rs`, `photocraft_engine`'s
//! `c0_sessions`): a stable id per document, `c0.activate`, `c0.close`, the tab filter `c0.tabs`, and
//! `c0.documentBytes`.
//!
//! They run through `control::handle` exactly as the page bridge's requests do. The tab filter is
//! checked through what the tab strip draws (the accessibility nodes of a real frame, in a Studio and
//! a Pro theme) and through the Ctrl+Tab shortcut, not only through the stored list. What the browser
//! adds (the page function, base64 over `window.__photocraftControl`) is not covered here.

use base64::Engine as _;
use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use photocraft_color::{ColorMode, PixelFormat, SampleType};
use photocraft_doc::{Document, Layer, LayerContent};
use photocraft_geom::{Rect, Size};
use photocraft_raster::Surface;
use photocraft_ui_egui::control::{ControlRequest, Outcome, handle};
use photocraft_ui_egui::theme::ThemeKind;
use photocraft_ui_egui::{PhotocraftApp, Services};
use serde_json::{Value, json};

fn services() -> Services {
    Services {
        import: Some(Box::new(|name: &str, bytes: &[u8], depth: usize| {
            photocraft_io::import_with_svg_group_depth(name, bytes, depth).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
        })),
        ..Default::default()
    }
}

fn new_app() -> PhotocraftApp {
    PhotocraftApp::new(photocraft_engine::Session::new(), services())
}

/// A document called `name`, `width` x `height`, with one raster layer per entry of `layers`.
fn document(name: &str, width: u32, height: u32, layers: &[&str]) -> Document {
    let mut doc = Document::new(name, Size::new(width, height), ColorMode::Rgb, SampleType::U8);
    for (n, layer) in layers.iter().enumerate() {
        let colour = [(n as u8 + 1) * 70, 40, 90, 255];
        let bytes: Vec<u8> = colour.repeat((width * height) as usize);
        let surface = Surface::from_interleaved(PixelFormat::RGBA8, Rect::new(0, 0, width as i32, height as i32), &bytes);
        doc.layers.push(Layer::new(*layer, LayerContent::Raster(surface)));
    }
    doc
}

fn open(app: &mut PhotocraftApp, name: &str, width: u32, height: u32, layers: &[&str]) -> u64 {
    app.session.open_document(document(name, width, height, layers), None);
    app.sync_views();
    app.session.documents().last().unwrap().uid
}

fn call(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (req, _rx) = ControlRequest::new(method, params);
    match handle(app, ctx, &req) {
        Outcome::Done(reply) => reply,
        _ => panic!("{method} did not answer at once"),
    }
}

fn ok(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let reply = call(app, ctx, method, params.clone());
    assert_eq!(reply["ok"], true, "{method} {params}: {reply}");
    reply["result"].clone()
}

fn error(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: Value) -> String {
    let reply = call(app, ctx, method, params.clone());
    assert_eq!(reply["ok"], false, "{method} {params} should fail: {reply}");
    reply["error"].as_str().unwrap_or_else(|| panic!("{method}: no error text in {reply}")).to_owned()
}

fn execute(app: &mut PhotocraftApp, ctx: &egui::Context, command: &str, params: Value) -> Value {
    ok(app, ctx, "engine.execute", json!({"command": command, "params": params}))
}

fn inspect(app: &mut PhotocraftApp, ctx: &egui::Context) -> Value {
    ok(app, ctx, "ui.inspect", json!({}))
}

/// `(name, id)` of every document in tab order, as `ui.inspect` lists them.
fn listed(app: &mut PhotocraftApp, ctx: &egui::Context) -> Vec<(String, u64)> {
    let state = inspect(app, ctx);
    state["session"]["documents"].as_array().unwrap().iter().map(|d| (d["name"].as_str().unwrap().to_owned(), d["id"].as_u64().expect("an id"))).collect()
}

fn id_of(list: &[(String, u64)], name: &str) -> u64 {
    list.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("{name} is not open: {list:?}")).1
}

fn unb64(text: &Value) -> Vec<u8> {
    let text = text.as_str().expect("base64 text");
    assert!(!text.starts_with("data:"), "no data: prefix");
    base64::engine::general_purpose::STANDARD.decode(text).expect("valid base64")
}

// ---------- ids ----------

#[test]
fn ids_are_listed_distinct_and_survive_moves_and_closes() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    for name in ["one", "two", "three"] {
        open(&mut app, name, 8, 8, &["a"]);
    }
    let first = listed(&mut app, &ctx);
    let mut ids: Vec<u64> = first.iter().map(|(_, id)| *id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3, "two documents never share an id: {first:?}");
    let (one, two, three) = (id_of(&first, "one"), id_of(&first, "two"), id_of(&first, "three"));
    // `session.inspect` carries the same ids.
    let via_engine = execute(&mut app, &ctx, "session.inspect", json!({}));
    assert_eq!(via_engine["documents"][1]["id"], two, "{via_engine}");

    // Moving tabs changes indices, not ids.
    execute(&mut app, &ctx, "document.move", json!({"document": 0, "to": 2}));
    let moved = listed(&mut app, &ctx);
    assert_eq!(moved.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["two", "three", "one"]);
    assert_eq!((id_of(&moved, "one"), id_of(&moved, "two"), id_of(&moved, "three")), (one, two, three));

    // Closing an earlier document shifts the indices of the others and keeps their ids.
    let closed = ok(&mut app, &ctx, "c0.close", json!({"id": two}));
    assert_eq!(closed, json!({"id": two, "closed": true, "prompt": false}));
    let after = listed(&mut app, &ctx);
    assert_eq!(after, vec![("three".to_owned(), three), ("one".to_owned(), one)]);

    // A closed id is not handed out again, not even to the same document opened again.
    let fresh = open(&mut app, "four", 8, 8, &["a"]);
    assert!(![one, two, three].contains(&fresh), "{fresh} reuses an id");
    let bytes = ok(&mut app, &ctx, "c0.documentBytes", json!({"id": three}));
    ok(&mut app, &ctx, "c0.close", json!({"id": three}));
    let reopened = ok(&mut app, &ctx, "c0.openFile", json!({"name": "three.pcraft", "bytes": bytes["bytes"]}));
    let reopened_id = reopened["id"].as_u64().expect("c0.openFile answers the id");
    assert!(![one, two, three, fresh].contains(&reopened_id), "the reopened document reuses id {reopened_id}");
    assert_eq!(reopened, json!({"document": 2, "id": reopened_id}));
    assert_eq!(listed(&mut app, &ctx).last().unwrap().1, reopened_id);
}

#[test]
fn c0_requests_take_an_id_for_a_document_that_is_not_active() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let wide = open(&mut app, "wide", 30, 10, &["a", "b"]);
    let tall = open(&mut app, "tall", 10, 40, &["a"]);
    assert_eq!(app.session.active().unwrap().uid, tall);

    let capture = ok(&mut app, &ctx, "c0.capture", json!({"scope": "document", "id": wide}));
    assert_eq!(capture["rect"], json!({"x": 0, "y": 0, "width": 30, "height": 10}), "{capture}");
    let own = ok(&mut app, &ctx, "c0.capture", json!({"scope": "document"}));
    assert_eq!(own["rect"], json!({"x": 0, "y": 0, "width": 10, "height": 40}));

    let layers_of = |app: &PhotocraftApp, id: u64| app.session.by_uid(id).unwrap().doc.layer_count();
    let before = (layers_of(&app, wide), layers_of(&app, tall));
    let png = {
        let image = photocraft_codecs::Image::from_u8(2, 2, photocraft_codecs::ChannelLayout::Rgba, [200u8, 10, 10, 255].repeat(4)).unwrap();
        photocraft_codecs::encode(&image, photocraft_codecs::Format::Png, &Default::default()).unwrap()
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    let placed = ok(&mut app, &ctx, "c0.place", json!({"png": b64, "name": "Generated", "id": wide}));
    assert!(placed["layer"].is_u64(), "{placed}");
    assert_eq!((layers_of(&app, wide), layers_of(&app, tall)), (before.0 + 1, before.1), "the layer went into the document the id names");
    assert_eq!(app.session.active().unwrap().uid, tall, "placing into another document leaves the active tab alone");

    let name = error(&mut app, &ctx, "c0.capture", json!({"scope": "document", "id": 999_999}));
    assert_eq!(name, "no document with id 999999");
    assert_eq!(error(&mut app, &ctx, "c0.capture", json!({"scope": "document", "id": "x"})), "`id` must be a whole document id");
}

// ---------- activate and close ----------

#[test]
fn activate_and_close_by_id_with_the_unsaved_changes_prompt() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let a = open(&mut app, "a", 8, 8, &["x"]);
    let b = open(&mut app, "b", 8, 8, &["x"]);
    let c = open(&mut app, "c", 8, 8, &["x"]);
    assert_eq!(app.session.active().unwrap().uid, c);

    let activated = ok(&mut app, &ctx, "c0.activate", json!({"id": a}));
    assert_eq!(activated, json!({"id": a, "document": 0}));
    assert_eq!(app.session.active().unwrap().uid, a);
    assert_eq!(inspect(&mut app, &ctx)["session"]["active"], 0);

    // A document with unsaved changes: the app's own prompt, nothing closes.
    ok(&mut app, &ctx, "c0.activate", json!({"id": b}));
    execute(&mut app, &ctx, "layer.new.layer", json!({}));
    assert!(app.session.by_uid(b).unwrap().is_dirty());
    let asked = ok(&mut app, &ctx, "c0.close", json!({"id": b}));
    assert_eq!(asked, json!({"id": b, "closed": false, "prompt": true}));
    assert!(app.session.by_uid(b).is_some(), "the document stays open behind the prompt");
    // With discard, no prompt and the changes are dropped.
    let dropped = ok(&mut app, &ctx, "c0.close", json!({"id": b, "discard": true}));
    assert_eq!(dropped, json!({"id": b, "closed": true, "prompt": false}));
    assert!(app.session.by_uid(b).is_none());

    // A clean document closes without asking, and the others keep their ids.
    let clean = ok(&mut app, &ctx, "c0.close", json!({"id": c}));
    assert_eq!(clean, json!({"id": c, "closed": true, "prompt": false}));
    assert_eq!(listed(&mut app, &ctx), vec![("a".to_owned(), a)]);

    for method in ["c0.activate", "c0.close", "c0.documentBytes"] {
        assert_eq!(error(&mut app, &ctx, method, json!({"id": b})), format!("no document with id {b}"), "{method}");
    }
    assert_eq!(error(&mut app, &ctx, "c0.close", json!({})), "`id` is required");
    assert_eq!(error(&mut app, &ctx, "c0.close", json!({"id": a, "discard": "yes"})), "`discard` must be a boolean");
}

// ---------- the tab filter ----------

fn harness(theme: ThemeKind, names: &[&'static str]) -> Harness<'static, PhotocraftApp> {
    let names = names.to_vec();
    let mut h = Harness::builder().with_size(vec2(1100.0, 700.0)).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, theme);
        let mut app = new_app();
        app.run("prefs.set", json!({"values": {"interface.theme": theme.id()}})).unwrap();
        for name in names {
            app.run("file.new", json!({"width": 40, "height": 30, "name": name})).unwrap();
        }
        app
    });
    h.run_steps(4);
    h
}

/// Whether the tab strip drew a tab for the document called `name` (the tab's accessibility node
/// carries the document's name).
fn tab_drawn(h: &Harness<'_, PhotocraftApp>, name: &str) -> bool {
    h.query_all_by_label_contains(name).next().is_some()
}

fn drawn(h: &Harness<'_, PhotocraftApp>, names: &[&'static str]) -> Vec<&'static str> {
    names.iter().copied().filter(|n| tab_drawn(h, n)).collect()
}

fn control(h: &mut Harness<'_, PhotocraftApp>, method: &str, params: Value) -> Value {
    let ctx = h.ctx.clone();
    let reply = call(h.state_mut(), &ctx, method, params.clone());
    h.run_steps(3);
    reply
}

fn inspected(h: &mut Harness<'_, PhotocraftApp>) -> Value {
    let ctx = h.ctx.clone();
    inspect(h.state_mut(), &ctx)
}

fn ctrl_tab(h: &mut Harness<'_, PhotocraftApp>, shift: bool) {
    let ctrl = egui::Modifiers { ctrl: true, command: !cfg!(target_os = "macos"), shift, ..egui::Modifiers::NONE };
    h.key_press_modifiers(ctrl, egui::Key::Tab);
    h.run_steps(2);
}

fn active_name(h: &Harness<'_, PhotocraftApp>) -> String {
    h.state().session.active().unwrap().doc.name.clone()
}

fn ids_of(h: &Harness<'_, PhotocraftApp>, names: &[&str]) -> Vec<u64> {
    names.iter().map(|n| h.state().session.documents().iter().find(|d| d.doc.name == *n).unwrap().uid).collect()
}

fn the_tab_filter_in(theme: ThemeKind) {
    const ALL: [&str; 4] = ["sess-alpha", "sess-beta", "sess-gamma", "sess-delta"];
    let mut h = harness(theme, &ALL);
    assert_eq!(drawn(&h, &ALL), ALL, "no filter: every tab is drawn");
    let [alpha, beta, gamma, delta] = ids_of(&h, &ALL)[..] else { panic!() };
    let (revisions, dirty): (Vec<u64>, Vec<bool>) = h.state().session.documents().iter().map(|d| (d.revision, d.is_dirty())).unzip();

    // Only the listed documents have tabs; the active one (delta) is among them.
    let shown = control(&mut h, "c0.tabs", json!({"visible": [beta, delta]}));
    assert_eq!(shown["ok"], true, "{shown}");
    assert_eq!(shown["result"], json!({"visible": [beta, delta], "active": delta}));
    assert_eq!(drawn(&h, &ALL), ["sess-beta", "sess-delta"], "the strip draws only the listed documents");
    assert_eq!(inspected(&mut h)["session"]["visible"], json!([beta, delta]));
    // Hidden documents stay open and untouched.
    assert_eq!(h.state().session.documents().len(), 4);
    let (now_revisions, now_dirty): (Vec<u64>, Vec<bool>) = h.state().session.documents().iter().map(|d| (d.revision, d.is_dirty())).unzip();
    assert_eq!((now_revisions, now_dirty), (revisions.clone(), dirty.clone()));

    // Ctrl+Tab and Ctrl+Shift+Tab cycle among the listed documents only, wrapping.
    assert_eq!(active_name(&h), "sess-delta");
    ctrl_tab(&mut h, false);
    assert_eq!(active_name(&h), "sess-beta", "Ctrl+Tab from the last listed tab wraps past the hidden ones");
    ctrl_tab(&mut h, false);
    assert_eq!(active_name(&h), "sess-delta", "and does not stop on the hidden gamma");
    ctrl_tab(&mut h, true);
    assert_eq!(active_name(&h), "sess-beta", "Ctrl+Shift+Tab goes back through the listed ones");
    ctrl_tab(&mut h, true);
    assert_eq!(active_name(&h), "sess-delta");

    // A hidden document cannot be activated by id.
    let refused = control(&mut h, "c0.activate", json!({"id": gamma}));
    assert_eq!(refused["error"], format!("document {gamma} is hidden by the tab filter"), "{refused}");

    // A list without the active document moves the active one to the first listed.
    let moved = control(&mut h, "c0.tabs", json!({"visible": [alpha, gamma]}));
    assert_eq!(moved["result"], json!({"visible": [alpha, gamma], "active": alpha}));
    assert_eq!(active_name(&h), "sess-alpha");
    assert_eq!(drawn(&h, &ALL), ["sess-alpha", "sess-gamma"]);

    // A document created while the filter is set joins it, and so does one opened from bytes.
    h.state_mut().run("file.new", json!({"width": 20, "height": 20, "name": "sess-epsilon"})).unwrap();
    h.run_steps(3);
    let epsilon = ids_of(&h, &["sess-epsilon"])[0];
    assert_eq!(h.state().session.visible(), Some(&[alpha, gamma, epsilon][..]));
    assert!(tab_drawn(&h, "sess-epsilon"), "the new document's tab is drawn");
    let bytes = control(&mut h, "c0.documentBytes", json!({"id": gamma}))["result"]["bytes"].clone();
    let opened = control(&mut h, "c0.openFile", json!({"name": "sess-zeta.pcraft", "bytes": bytes}));
    let zeta = opened["result"]["id"].as_u64().unwrap();
    assert_eq!(h.state().session.visible(), Some(&[alpha, gamma, epsilon, zeta][..]));
    assert_eq!(h.state().session.documents().len(), 6);

    // Closing a listed document leaves the list without it, and the active tab on a listed one.
    control(&mut h, "c0.close", json!({"id": zeta}));
    control(&mut h, "c0.activate", json!({"id": epsilon}));
    control(&mut h, "c0.close", json!({"id": epsilon}));
    assert_eq!(h.state().session.visible(), Some(&[alpha, gamma][..]));
    assert!(h.state().session.visible().unwrap().contains(&h.state().session.active().unwrap().uid), "the active document is a listed one");

    // An unknown id is a named error and changes nothing; an empty list shows no tab at all.
    let unknown = control(&mut h, "c0.tabs", json!({"visible": [alpha, 424_242]}));
    assert_eq!(unknown["error"], "no document with id 424242");
    assert_eq!(h.state().session.visible(), Some(&[alpha, gamma][..]));
    let none = control(&mut h, "c0.tabs", json!({"visible": []}));
    assert_eq!(none["result"]["visible"], json!([]));
    assert!(drawn(&h, &ALL).is_empty(), "an empty list draws no document tab: {:?}", drawn(&h, &ALL));
    // Without `visible` the request only reports.
    let report = control(&mut h, "c0.tabs", json!({}));
    assert_eq!(report["result"]["visible"], json!([]));

    // null shows every tab again.
    let all = control(&mut h, "c0.tabs", json!({"visible": null}));
    assert_eq!(all["result"]["visible"], Value::Null);
    assert_eq!(all["result"]["active"], h.state().session.active().unwrap().uid);
    assert_eq!(drawn(&h, &ALL), ALL, "null: every tab is drawn again");
    assert_eq!(inspected(&mut h)["session"]["visible"], Value::Null);
    ctrl_tab(&mut h, false);
    ctrl_tab(&mut h, false);
    assert!(ALL.contains(&active_name(&h).as_str()));
}

#[test]
fn the_tab_filter_in_the_pro_theme() {
    the_tab_filter_in(ThemeKind::Pro);
}

#[test]
fn the_tab_filter_in_the_studio_theme() {
    the_tab_filter_in(ThemeKind::Studio);
}

#[test]
fn the_tab_filter_applies_to_window_arrange() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let ids: Vec<u64> = ["p", "q", "r"].iter().map(|n| open(&mut app, n, 8, 8, &["a"])).collect();
    app.session.set_visible(Some(vec![ids[0], ids[2]])).unwrap();
    app.session.set_active(0);
    assert_eq!(app.session.arrange_order(0), [0, 2], "only the listed documents are tiled");
    assert_eq!(app.session.visible_indices(), [0, 2]);
    assert_eq!(app.session.cycle_visible(0, true), 2);
    assert_eq!(app.session.cycle_visible(2, true), 0);
    assert_eq!(app.session.cycle_visible(0, false), 2);
    ok(&mut app, &ctx, "c0.tabs", json!({"visible": null}));
    assert_eq!(app.session.arrange_order(1), [1, 2, 0]);
}

// ---------- document bytes ----------

#[test]
fn document_bytes_round_trip_without_touching_the_source() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let id = open(&mut app, "layered", 24, 16, &["Sky", "Hills", "Sun"]);
    // Some history, so there is something not to disturb.
    execute(&mut app, &ctx, "layer.new.layer", json!({}));
    let st = app.session.by_uid(id).unwrap();
    let (revision, dirty, past) = (st.revision, st.is_dirty(), st.history.past_len());
    let names: Vec<String> = st.doc.layers.iter().map(|l| l.name.clone()).collect();
    assert!(dirty && past > 0);

    let reply = ok(&mut app, &ctx, "c0.documentBytes", json!({"id": id, "format": "pcraft"}));
    assert_eq!(reply["name"], "layered");
    assert_eq!(reply["format"], "pcraft");
    assert_eq!(reply["revision"], revision);
    let keys: Vec<&String> = reply.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["bytes", "format", "name", "revision"]);
    let bytes = unb64(&reply["bytes"]);
    assert!(photocraft_format::is_pcraft(&bytes), "the native layered format");

    // Read-only: the dirty flag, the history and the revision are as they were.
    let st = app.session.by_uid(id).unwrap();
    assert_eq!((st.revision, st.is_dirty(), st.history.past_len()), (revision, dirty, past));
    // The default (no `format`, no `id`: the active document) gives the same bytes.
    let again = ok(&mut app, &ctx, "c0.documentBytes", json!({}));
    assert_eq!(unb64(&again["bytes"]), bytes);

    // Reopened through c0.openFile: same size, same layers.
    let reopened = ok(&mut app, &ctx, "c0.openFile", json!({"name": "layered.pcraft", "bytes": reply["bytes"]}));
    let copy = app.session.by_uid(reopened["id"].as_u64().unwrap()).unwrap();
    assert_eq!((copy.doc.size.width, copy.doc.size.height), (24, 16));
    assert_eq!(copy.doc.layers.iter().map(|l| l.name.clone()).collect::<Vec<_>>(), names);
    assert!(!copy.is_dirty(), "a file just opened has no unsaved changes");

    // Unknown ids and formats are named errors.
    assert_eq!(error(&mut app, &ctx, "c0.documentBytes", json!({"id": 777_777})), "no document with id 777777");
    assert_eq!(error(&mut app, &ctx, "c0.documentBytes", json!({"id": id, "format": "psd"})), "`format` must be \"pcraft\"");
    let none = call(&mut new_app(), &ctx, "c0.documentBytes", json!({}));
    assert_eq!(none["error"], "no document open");
}
