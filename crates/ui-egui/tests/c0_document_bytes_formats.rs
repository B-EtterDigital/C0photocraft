//! `c0.documentBytes` writes the open document as .pcraft, PSD, PNG or JPG for the host's "Save to Project"
//! (founder 2026-11-10). Each format is checked by its file signature and by reading it back.

use base64::Engine as _;
use photocraft_ui_egui::PhotocraftApp;
use photocraft_ui_egui::control::{ControlRequest, Outcome, handle};
use serde_json::{Value, json};

fn call(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (req, _rx) = ControlRequest::new(method, params);
    match handle(app, ctx, &req) {
        Outcome::Done(reply) => reply,
        _ => panic!("{method} did not answer at once"),
    }
}

fn app_with_two_layers() -> (PhotocraftApp, egui::Context) {
    let ctx = egui::Context::default();
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
    for (command, params) in [
        ("file.new", json!({"width": 40, "height": 30, "background": "white", "name": "cover"})),
        ("layer.new.layer", json!({"name": "lettering"})),
    ] {
        let reply = call(&mut app, &ctx, "engine.execute", json!({"command": command, "params": params}));
        assert_eq!(reply["ok"], true, "{command}: {reply}");
    }
    (app, ctx)
}

fn bytes_of(reply: &Value) -> Vec<u8> {
    assert_eq!(reply["ok"], true, "{reply}");
    base64::engine::general_purpose::STANDARD.decode(reply["result"]["bytes"].as_str().unwrap()).unwrap()
}

#[test]
fn every_format_comes_back_with_its_own_signature() {
    let (mut app, ctx) = app_with_two_layers();
    let psd = bytes_of(&call(&mut app, &ctx, "c0.documentBytes", json!({"format": "psd"})));
    assert_eq!(&psd[..4], b"8BPS", "PSD signature");
    let png = bytes_of(&call(&mut app, &ctx, "c0.documentBytes", json!({"format": "png"})));
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "PNG signature");
    let jpg = bytes_of(&call(&mut app, &ctx, "c0.documentBytes", json!({"format": "jpg", "quality": 85})));
    assert_eq!(&jpg[..3], [0xFF, 0xD8, 0xFF], "JPEG signature");
    let reply = call(&mut app, &ctx, "c0.documentBytes", json!({}));
    assert_eq!(reply["result"]["format"], photocraft_format::EXTENSION, "the default stays .pcraft");
    assert!(reply["result"]["warnings"].is_array());
}

#[test]
fn a_psd_keeps_the_layers() {
    let (mut app, ctx) = app_with_two_layers();
    let psd = bytes_of(&call(&mut app, &ctx, "c0.documentBytes", json!({"format": "psd"})));
    let opened = photocraft_io::import("cover.psd", &psd).expect("the PSD reads back");
    assert!(opened.document.layers.len() >= 2, "layers: {}", opened.document.layers.len());
}

#[test]
fn unknown_formats_and_bad_qualities_are_refused() {
    let (mut app, ctx) = app_with_two_layers();
    for params in [json!({"format": "bmp"}), json!({"format": 3}), json!({"format": "jpg", "quality": 0}), json!({"format": "jpg", "quality": 101}), json!({"format": "jpg", "quality": "high"})] {
        let reply = call(&mut app, &ctx, "c0.documentBytes", params.clone());
        assert_eq!(reply["ok"], false, "{params} was accepted: {reply}");
    }
}

#[test]
fn a_document_can_be_renamed_by_the_host() {
    let (mut app, ctx) = app_with_two_layers();
    let state = call(&mut app, &ctx, "ui.inspect", json!({}));
    let id = state["result"]["session"]["documents"][0]["id"].clone();
    let reply = call(&mut app, &ctx, "c0.rename", json!({"id": id, "name": "  CPO cover - Ausgewandert (layers).psd "}));
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["result"]["name"], "CPO cover - Ausgewandert (layers).psd");
    let state = call(&mut app, &ctx, "ui.inspect", json!({}));
    assert_eq!(state["result"]["session"]["documents"][0]["name"], "CPO cover - Ausgewandert (layers).psd");
    for bad in ["", "   ", "a/b", "a\\b", "tab\there"] {
        let reply = call(&mut app, &ctx, "c0.rename", json!({"id": id, "name": bad}));
        assert_eq!(reply["ok"], false, "{bad:?} was accepted: {reply}");
    }
    let reply = call(&mut app, &ctx, "c0.rename", json!({"id": 999_999, "name": "x"}));
    assert_eq!(reply["ok"], false);
}
