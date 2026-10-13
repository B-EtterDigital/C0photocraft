//! What a control request costs in egui passes: the facts the page bridge
//! (`apps/photocraft-web/src/page_control`) builds on when it answers a request in a pass of its
//! own instead of waiting for the display's next frame.
//!
//! A pass here is `Harness::step`: the app's `logic` (which drains the control queue first) and its
//! `ui`, as eframe's web runner runs them, without painting. Whether the display then paints is the
//! browser's business and is not covered here.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use photocraft_ui_egui::PhotocraftApp;
use photocraft_ui_egui::control::ControlRequest;
use serde_json::{Value, json};

type Harness = egui_kittest::Harness<'static, PhotocraftApp>;

/// The app with one document and a control channel, run until it asks for no more frames.
fn idle_app() -> (Harness, Sender<ControlRequest>) {
    let (tx, rx) = channel::<ControlRequest>();
    let mut h: Harness = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width": 64, "height": 64})).unwrap();
        app.with_control(rx)
    });
    h.run_steps(8);
    let steps = h.try_run().expect("an untouched app stops asking for frames");
    assert!(steps <= 4, "the app was idle after {steps} more passes");
    (h, tx)
}

fn send(tx: &Sender<ControlRequest>, method: &str, params: Value) -> Receiver<Value> {
    let (req, rx) = ControlRequest::new(method, params);
    tx.send(req).unwrap();
    rx
}

/// How soon the last pass wants the next one: zero is "at once" (the display would paint again).
fn wants_next(h: &Harness) -> Duration {
    h.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay
}

fn frame(h: &Harness) -> u64 {
    h.state().frame
}

/// The requests the bridge treats as reads (`page_control::reads_only`).
fn reads() -> Vec<(&'static str, Value)> {
    vec![
        ("ui.inspect", json!({})),
        ("ui.menu.list", json!({})),
        ("engine.commands", json!({})),
        ("engine.execute", json!({"command": "document.inspect"})),
        ("engine.execute", json!({"command": "session.inspect"})),
        ("engine.execute", json!({"command": "document.pixel", "params": {"x": 1, "y": 1}})),
    ]
}

/// One pass answers a reading request, and that pass asks for no further frame: answered from an
/// idle app, a read costs no painted frame.
#[test]
fn a_read_is_answered_by_one_pass_that_asks_for_no_frame() {
    let (mut h, tx) = idle_app();
    for (method, params) in reads() {
        let before = frame(&h);
        let reply = send(&tx, method, params.clone());
        assert!(reply.try_recv().is_err(), "{method}: nothing answers before a pass runs");
        h.step();
        let reply = reply.try_recv().unwrap_or_else(|_| panic!("{method} {params}: not answered by the pass"));
        assert_eq!(reply["ok"], true, "{method} {params}: {reply}");
        assert_eq!(frame(&h), before + 1, "{method}: exactly one pass ran");
        assert_ne!(wants_next(&h), Duration::ZERO, "{method} {params}: the pass that answered a read asked for another frame at once");
    }
}

/// The bridge used to wake the app for every request (`request_repaint`). egui turns one such wake
/// into two passes, which on a visible window are two painted frames per request, reads included.
#[test]
fn waking_the_app_for_a_read_costs_two_passes() {
    let (mut h, tx) = idle_app();
    let reply = send(&tx, "ui.inspect", json!({}));
    h.ctx.request_repaint();
    let passes = h.try_run().expect("the app settles after a read");
    assert_eq!(reply.try_recv().unwrap()["ok"], true);
    eprintln!("passes after request_repaint + ui.inspect: {passes}");
    assert!(passes >= 2, "egui repaints twice per request_repaint; measured {passes}");
}

/// What a request reads is current: a change made by the request before it is seen by the next
/// one, each in its own pass, with no painted frame in between.
#[test]
fn a_change_is_seen_by_the_next_request_one_pass_later() {
    let (mut h, tx) = idle_app();
    let layers = |reply: &Value| reply["result"]["document"]["layers"].as_array().map(Vec::len).unwrap_or(0);
    let first = send(&tx, "ui.inspect", json!({}));
    h.step();
    let first = first.try_recv().unwrap();
    let before = layers(&first);
    assert!(before >= 1, "{first}");

    let added = send(&tx, "engine.execute", json!({"command": "layer.new.layer"}));
    h.step();
    assert_eq!(added.try_recv().unwrap()["ok"], true);
    let tool = send(&tx, "ui.set", json!({"tool": "brush", "panels": {"layers": false}}));
    h.step();
    assert_eq!(tool.try_recv().unwrap()["ok"], true);
    let second = send(&tx, "ui.inspect", json!({}));
    h.step();
    let second = second.try_recv().unwrap();
    assert_eq!(layers(&second), before + 1, "the new layer is in the next request's reply");
    assert_eq!(second["result"]["tool"], "Brush", "{}", second["result"]["tool"]);
    assert_eq!(second["result"]["panels"]["layers"], false);
    assert_eq!(second["result"]["frame"].as_u64().unwrap(), first["result"]["frame"].as_u64().unwrap() + 3, "three requests, three passes");
}

/// Several requests waiting when a pass starts are all answered by it, in order.
#[test]
fn one_pass_answers_every_waiting_request_in_order() {
    let (mut h, tx) = idle_app();
    let set = send(&tx, "ui.set", json!({"tool": "eraser"}));
    let read = send(&tx, "ui.inspect", json!({}));
    h.step();
    assert_eq!(set.try_recv().unwrap()["ok"], true);
    assert_eq!(read.try_recv().unwrap()["result"]["tool"], "Eraser");
}

/// Synthetic input is played one step per frame, so `ui.click` is not answered by its first pass,
/// and the app asks for the next frame at once: that request is what the bridge's pump goes by to
/// give a waiting request more passes. (The steps themselves are fed in through eframe's
/// `raw_input_hook`, which this harness does not call, so the rest of the click is not run here.)
#[test]
fn synthetic_input_is_not_answered_by_one_pass_and_asks_for_the_next() {
    let (mut h, tx) = idle_app();
    let canvas = h.state().last_canvas_rect.center();
    let click = send(&tx, "ui.click", json!({"x": canvas.x, "y": canvas.y}));
    h.step();
    assert!(click.try_recv().is_err(), "a click is not through after its first pass");
    assert_eq!(wants_next(&h), Duration::ZERO, "the app waits for input it queued without asking for a frame");
    assert!(h.ctx.has_requested_repaint(), "the pump reads this to keep the passes coming");
}
