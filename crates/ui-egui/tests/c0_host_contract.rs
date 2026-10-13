//! The facts the C0VIBE host shell relies on, so a pull from upstream cannot silently break it.
//!
//! The host (PhotoCraft embedded as a module of another app, see `c0/photocraft-host/README.md`)
//! steers the editor through the control protocol and draws its own chrome round it. Each test
//! below is one thing the host's code names or reads. When one fails after a pull, upstream renamed
//! or removed something the host uses: fix the host (or carry the difference) in the same change
//! that takes the pull. Do not loosen the test to make the pull go through.
//!
//! Not asserted here, because no Rust test can: that the page function
//! `window.__photocraftControl` exists and resolves (browser only), and that `ui.set {themeTokens}`
//! is accepted through it (the field is taken out of the request by the browser bridge,
//! `apps/photocraft-web/src/page_control/bridge.rs`; what is asserted is the function the bridge
//! hands the tokens to, and that the app itself still refuses the field).

use std::collections::BTreeSet;

use egui_kittest::kittest::Queryable as _;
use photocraft_ui_egui::PhotocraftApp;
use photocraft_ui_egui::control::{ControlRequest, Outcome, handle};
use photocraft_ui_egui::theme::{ThemeKind, Tokens, host};
use serde_json::{Value, json};

/// The switches of `ui.inspect().panels` the host sets with `ui.set {panels}`.
const PANELS: [&str; 14] = [
    "menu_bar",
    "toolbar",
    "toolbar_double",
    "toolbar_floating",
    "layers",
    "history",
    "properties",
    "color",
    "navigator",
    "options_bar",
    "status_bar",
    "dock",
    "rail",
    "community_links",
];

/// The colour tokens the host sends in `ui.set {themeTokens}`.
const COLOUR_TOKENS: [&str; 26] = [
    "chrome",
    "canvas",
    "canvas_dot",
    "dock",
    "card",
    "card_border",
    "field",
    "field_border",
    "hover",
    "pressed",
    "text",
    "text_dim",
    "text_faint",
    "icon",
    "accent",
    "accent_soft",
    "accent_border",
    "accent_text",
    "separator",
    "shadow",
    "primary_bg",
    "primary_text",
    "danger",
    "tab_strip",
    "row_selected",
    "scrim",
];

/// The radius tokens the host sends with them.
const RADIUS_TOKENS: [&str; 3] = ["radius_sm", "radius", "radius_lg"];

fn new_app() -> PhotocraftApp {
    PhotocraftApp::new(photocraft_engine::Session::new(), Default::default())
}

fn call(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (req, _rx) = ControlRequest::new(method, params);
    match handle(app, ctx, &req) {
        Outcome::Done(reply) => reply,
        _ => panic!("{method} did not answer at once"),
    }
}

/// The `result` of a request that must succeed.
fn ok(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let reply = call(app, ctx, method, params.clone());
    assert_eq!(reply["ok"], true, "{method} {params}: {reply}");
    reply["result"].clone()
}

fn execute(app: &mut PhotocraftApp, ctx: &egui::Context, command: &str, params: Value) -> Value {
    ok(app, ctx, "engine.execute", json!({"command": command, "params": params}))
}

fn inspect(app: &mut PhotocraftApp, ctx: &egui::Context) -> Value {
    ok(app, ctx, "ui.inspect", json!({}))
}

fn has_keys(value: &Value, keys: &[&str], what: &str) {
    let object = value.as_object().unwrap_or_else(|| panic!("{what} is not an object: {value}"));
    for key in keys {
        assert!(object.contains_key(*key), "{what} has no `{key}` (has: {:?})", object.keys().collect::<Vec<_>>());
    }
}

/// (a) Every panel switch the host uses is a boolean of `ui.inspect().panels` and is set by
/// `ui.set {panels: {<name>: bool}}`, one at a time, both ways.
#[test]
fn a_panel_switches_are_set_through_ui_set() {
    let ctx = egui::Context::default();
    for name in PANELS {
        let mut app = new_app();
        let before = inspect(&mut app, &ctx)["panels"][name].as_bool().unwrap_or_else(|| panic!("ui.inspect().panels has no boolean `{name}`"));
        for value in [!before, before] {
            ok(&mut app, &ctx, "ui.set", json!({"panels": {name: value}}));
            assert_eq!(inspect(&mut app, &ctx)["panels"][name], value, "ui.set {{panels: {{{name}: {value}}}}} did not take");
        }
    }
    // All at once, as the host does when it mounts the editor.
    let mut app = new_app();
    let all: serde_json::Map<String, Value> = PANELS.iter().map(|name| (name.to_string(), json!(*name == "toolbar"))).collect();
    ok(&mut app, &ctx, "ui.set", json!({"panels": all}));
    let panels = inspect(&mut app, &ctx)["panels"].clone();
    for name in PANELS {
        assert_eq!(panels[name], name == "toolbar", "{name} after one ui.set of every switch");
    }
}

/// (b) The two presets the host picks between, and the palette it lays over them: exactly these
/// token names, every one applied, none unknown.
#[test]
fn b_studio_themes_and_the_host_palette() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    for (id, kind) in [("studio", ThemeKind::Studio), ("studioLight", ThemeKind::StudioLight)] {
        assert_eq!(kind.id(), id, "the preset's protocol name");
        ok(&mut app, &ctx, "ui.set", json!({"theme": id}));
        assert_eq!(app.ui.theme, kind);
        assert_eq!(inspect(&mut app, &ctx)["theme"], serde_json::to_value(kind).unwrap());
    }

    let mut palette = serde_json::Map::new();
    for (i, name) in COLOUR_TOKENS.iter().enumerate() {
        palette.insert(name.to_string(), json!(format!("#{:02x}{:02x}{:02x}", 10 + i, 40 + i, 70 + i)));
    }
    for (i, name) in RADIUS_TOKENS.iter().enumerate() {
        palette.insert(name.to_string(), json!(3.0 + i as f64));
    }
    assert_eq!(palette.len(), 29);
    PhotocraftApp::setup_context(&ctx, ThemeKind::Studio);
    let report = host::set(&ctx, &Value::Object(palette.clone())).expect("the palette is an object of tokens");
    let wanted: BTreeSet<&str> = COLOUR_TOKENS.iter().chain(RADIUS_TOKENS.iter()).copied().collect();
    let applied: BTreeSet<&str> = report.applied.iter().map(String::as_str).collect();
    assert_eq!(applied, wanted, "tokens applied");
    assert!(report.unknown.is_empty(), "tokens upstream no longer has: {:?}", report.unknown);
    assert!(report.invalid.is_empty(), "tokens whose value was refused: {:?}", report.invalid);
    // They are in force, not only reported.
    let tokens = Tokens::get(&ctx);
    assert_eq!(tokens.chrome, egui::Color32::from_rgb(10, 40, 70), "chrome is the palette's");
    assert_eq!(tokens.scrim, egui::Color32::from_rgb(35, 65, 95), "scrim is the palette's");
    assert_eq!((tokens.radius_sm, tokens.radius, tokens.radius_lg), (3.0, 4.0, 5.0));
    // The reply shape the bridge passes on.
    has_keys(&report.to_json(), &["applied", "unknown", "invalid"], "the themeTokens report");
    // A name that is not a token is reported, not swallowed.
    palette.insert("not_a_token".into(), json!("#000000"));
    assert_eq!(host::set(&ctx, &Value::Object(palette)).unwrap().unknown, ["not_a_token"]);
    assert!(host::set(&ctx, &Value::Null).unwrap().applied.is_empty(), "null clears the palette");

    // The app's own `ui.set` does not know the field: the bridge has to keep taking it out.
    let reply = call(&mut app, &ctx, "ui.set", json!({"themeTokens": {"chrome": "#000000"}}));
    assert_eq!(reply["ok"], false, "upstream now has a `themeTokens` of its own; the bridge's field collides with it: {reply}");
}

/// (c) The dock order the host asks for is accepted on a workspace nobody touched.
#[test]
fn c_dock_order_is_accepted_on_an_untouched_workspace() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let order = json!(["color", "properties", "layers"]);
    ok(&mut app, &ctx, "ui.set", json!({"dock": {"order": order}}));
    assert_eq!(inspect(&mut app, &ctx)["dock"]["order"], order);
}

/// (d) What the host reads from `ui.inspect`.
#[test]
fn d_ui_inspect_has_what_the_host_reads() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let state = inspect(&mut app, &ctx);
    has_keys(&state, &["tool", "frame", "panels", "dock", "docking", "document", "session"], "ui.inspect");
    assert!(state["tool"].is_string(), "tool: {}", state["tool"]);
    assert!(state["frame"].is_u64(), "frame: {}", state["frame"]);
    assert!(state["panels"].is_object());
    assert!(state["dock"]["order"].is_array(), "dock.order: {}", state["dock"]);
    assert!(state["document"].is_null(), "no document open: {}", state["document"]);
    assert!(state["session"]["active"].is_null(), "session.active with no document: {}", state["session"]["active"]);
    assert_eq!(state["session"]["documents"], json!([]));

    execute(&mut app, &ctx, "file.new", json!({"width": 320, "height": 200, "background": "white"}));
    let state = inspect(&mut app, &ctx);
    let document = &state["document"];
    has_keys(document, &["revision", "name", "width", "height", "canUndo", "canRedo", "history", "layers", "activeLayer"], "ui.inspect().document");
    assert_eq!((document["width"].as_u64(), document["height"].as_u64()), (Some(320), Some(200)));
    assert!(document["revision"].is_u64() && document["name"].is_string());
    assert!(document["canUndo"].is_boolean() && document["canRedo"].is_boolean());
    assert!(document["history"].is_array() && document["layers"].is_array());
    assert!(document["activeLayer"].is_u64(), "activeLayer: {}", document["activeLayer"]);
    assert_eq!(state["session"]["active"], 0);
    let listed = state["session"]["documents"].as_array().expect("session.documents");
    assert_eq!(listed.len(), 1);
    has_keys(&listed[0], &["index", "name", "width", "height", "dirty"], "ui.inspect().session.documents[0]");
    assert_eq!((listed[0]["index"].as_u64(), listed[0]["width"].as_u64(), listed[0]["height"].as_u64()), (Some(0), Some(320), Some(200)));
    assert!(listed[0]["name"].is_string() && listed[0]["dirty"].is_boolean());
}

/// (e) The engine commands the host runs, by id and with the parameters it sends.
#[test]
fn e_engine_commands_the_host_runs() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let listed = ok(&mut app, &ctx, "engine.commands", json!({}));
    let ids: BTreeSet<&str> = listed.as_array().expect("engine.commands lists commands").iter().filter_map(|c| c["id"].as_str()).collect();
    for id in [
        "file.new",
        "file.close",
        "document.activate",
        "document.inspect",
        "session.inspect",
        "edit.undo",
        "edit.redo",
        "layer.new.layer",
        "layer.select",
        "layer.setProps",
    ] {
        assert!(ids.contains(id), "engine command `{id}` is gone");
    }

    execute(&mut app, &ctx, "file.new", json!({"width": 64, "height": 48, "background": "white"}));
    execute(&mut app, &ctx, "file.new", json!({"width": 32, "height": 32, "background": "transparent"}));
    let session = execute(&mut app, &ctx, "session.inspect", json!({}));
    assert_eq!(session["active"], 1, "the new document is the active one");
    assert_eq!(session["documents"].as_array().map(Vec::len), Some(2));

    execute(&mut app, &ctx, "document.activate", json!({"document": 0}));
    let document = execute(&mut app, &ctx, "document.inspect", json!({}));
    assert_eq!((document["width"].as_u64(), document["height"].as_u64()), (Some(64), Some(48)), "document.activate {{document: 0}}");
    let background = document["activeLayer"].as_u64().expect("activeLayer");

    execute(&mut app, &ctx, "layer.new.layer", json!({}));
    let document = execute(&mut app, &ctx, "document.inspect", json!({}));
    assert_eq!(document["layers"].as_array().map(Vec::len), Some(2));
    let added = document["activeLayer"].as_u64().expect("activeLayer");
    assert_ne!(added, background, "the new layer is the active one");

    execute(&mut app, &ctx, "layer.select", json!({"layer": background}));
    assert_eq!(execute(&mut app, &ctx, "document.inspect", json!({}))["activeLayer"], background);
    execute(&mut app, &ctx, "layer.setProps", json!({"layer": added, "name": "Host layer", "visible": false, "opacity": 0.5}));
    let document = execute(&mut app, &ctx, "document.inspect", json!({}));
    let layer = document["layers"].as_array().unwrap().iter().find(|l| l["id"] == added).expect("layers are listed with their id");
    assert_eq!((&layer["name"], &layer["visible"]), (&json!("Host layer"), &json!(false)), "{layer}");

    assert_eq!(document["canUndo"], true);
    execute(&mut app, &ctx, "edit.undo", json!({}));
    assert_eq!(execute(&mut app, &ctx, "document.inspect", json!({}))["canRedo"], true);
    execute(&mut app, &ctx, "edit.redo", json!({}));

    execute(&mut app, &ctx, "file.close", json!({"document": 1}));
    let session = execute(&mut app, &ctx, "session.inspect", json!({}));
    assert_eq!(session["documents"].as_array().map(Vec::len), Some(1), "file.close {{document: 1}} closed one document: {session}");
    assert_eq!(session["documents"][0]["width"], 64, "and it was the second one");
}

/// (f) The menu list the host builds its own menus from.
#[test]
fn f_ui_menu_list_entries() {
    let ctx = egui::Context::default();
    let mut app = new_app();
    let menu = ok(&mut app, &ctx, "ui.menu.list", json!({}));
    let entries = menu.as_array().expect("ui.menu.list is a list");
    assert!(entries.len() > 100, "only {} menu entries", entries.len());
    for entry in entries {
        has_keys(entry, &["id", "label", "path"], "a ui.menu.list entry");
        assert!(entry["id"].is_string() && entry["label"].is_string() && entry["path"].is_array(), "{entry}");
    }
    assert!(entries.iter().any(|e| e["id"] == "file.export.quickExportAsPng"), "the menu no longer has file.export.quickExportAsPng");
}

/// (g) The Studio themes draw no Home button in the options bar (the host has its own way home).
/// Upstream draws it for themes whose `Tokens.pro` is set.
#[test]
fn g_studio_themes_draw_no_home_button() {
    let shows_home = |kind: ThemeKind| {
        let mut h: egui_kittest::Harness<'static, PhotocraftApp> =
            egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_max_steps(64).build_eframe(move |cc| {
                PhotocraftApp::setup_context(&cc.egui_ctx, kind);
                let mut app = new_app();
                app.run("file.new", json!({"width": 64, "height": 64})).unwrap();
                app.set_theme(&cc.egui_ctx, kind);
                app
            });
        h.run_steps(6);
        assert_eq!(h.state().ui.theme, kind);
        h.query_all_by_label("Home").count() > 0
    };
    assert!(shows_home(ThemeKind::Pro), "the probe finds the Home button where upstream draws it");
    for kind in [ThemeKind::Studio, ThemeKind::StudioLight] {
        assert!(!Tokens::for_kind(kind).pro, "{kind:?} became a Pro theme");
        assert!(!shows_home(kind), "{kind:?} draws a Home button");
    }
}
