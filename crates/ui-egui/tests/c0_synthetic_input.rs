//! Synthetic input over the control channel, as the page bridge plays it
//! (`apps/photocraft-web/src/page_control`): the request is read by one pass, which queues the
//! input; every later pass plays one step of it; the pass that plays the last step answers.
//!
//! eframe's `raw_input_hook` is what feeds a step into a pass. This harness does not call it, so
//! each pass is preceded by what the hook does for synthetic input,
//! `PhotocraftApp::take_synthetic_step`, handed to the harness as events. Whether eframe's web
//! runner really calls the hook in an unpainted pass (it does in 0.36.2: `AppRunner::logic`) is
//! not covered here.

use std::sync::mpsc::{Receiver, Sender, channel};

use photocraft_ui_egui::PhotocraftApp;
use photocraft_ui_egui::control::ControlRequest;
use serde_json::{Value, json};

type Harness = egui_kittest::Harness<'static, PhotocraftApp>;

fn app() -> (Harness, Sender<ControlRequest>) {
    let (tx, rx) = channel::<ControlRequest>();
    let mut h: Harness = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width": 64, "height": 64})).unwrap();
        app.with_control(rx)
    });
    h.run_steps(8);
    h.try_run().expect("an untouched app stops asking for frames");
    (h, tx)
}

fn send(tx: &Sender<ControlRequest>, method: &str, params: Value) -> Receiver<Value> {
    let (req, rx) = ControlRequest::new(method, params);
    tx.send(req).unwrap();
    rx
}

/// One pass, with the synthetic step the hook would add. Returns the events it fed.
fn pass(h: &mut Harness) -> Vec<egui::Event> {
    let events = h.state_mut().take_synthetic_step();
    for event in &events {
        h.event(event.clone());
    }
    h.step();
    events
}

/// Passes until `rx` has its reply, as the bridge's `drive` runs them; at most `limit` after the
/// first. Returns the reply and the events fed on the way.
fn drive(h: &mut Harness, rx: &Receiver<Value>, limit: usize) -> (Value, Vec<egui::Event>) {
    let mut fed = Vec::new();
    pass(h);
    for _ in 0..limit {
        if let Ok(reply) = rx.try_recv() {
            return (reply, fed);
        }
        fed.extend(pass(h));
    }
    (rx.try_recv().unwrap_or_else(|_| panic!("no reply after {} passes", limit + 1)), fed)
}

#[test]
fn a_move_is_queued_by_the_pass_that_reads_it_and_played_and_answered_by_the_next() {
    let (mut h, tx) = app();
    let target = egui::pos2(321.0, 222.0);
    let reply = send(&tx, "ui.move", json!({"x": target.x, "y": target.y}));
    pass(&mut h);
    assert!(reply.try_recv().is_err(), "the pass that reads the request only queues the input");
    assert!(h.ctx.has_requested_repaint(), "and asks for the next pass");

    let fed = pass(&mut h);
    assert_eq!(fed, vec![egui::Event::PointerMoved(target)], "the next pass is fed the queued move");
    let reply = reply.try_recv().expect("the pass that played the last step answers");
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(h.ctx.input(|i| i.pointer.latest_pos()), Some(target), "the app saw the pointer there");
}

#[test]
fn a_click_is_played_in_one_step_per_click_and_answered_after_the_last() {
    for count in [1u64, 2, 3] {
        let (mut h, tx) = app();
        let at = h.state().last_canvas_rect.center();
        let reply = send(&tx, "ui.click", json!({"x": at.x, "y": at.y, "count": count}));
        let (reply, fed) = drive(&mut h, &reply, count as usize + 1);
        assert_eq!(reply["ok"], true, "count {count}: {reply}");
        let presses = fed.iter().filter(|e| matches!(e, egui::Event::PointerButton { pressed: true, .. })).count();
        let releases = fed.iter().filter(|e| matches!(e, egui::Event::PointerButton { pressed: false, .. })).count();
        assert_eq!((presses, releases), (count as usize, count as usize), "every click reaches the app");
        assert!(h.state_mut().take_synthetic_step().is_empty(), "nothing is left queued");
    }
}

#[test]
fn a_key_and_typed_text_are_played_and_answered() {
    let (mut h, tx) = app();
    let key = send(&tx, "ui.key", json!({"key": "B"}));
    let (reply, fed) = drive(&mut h, &key, 2);
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(fed.iter().any(|e| matches!(e, egui::Event::Key { key: egui::Key::B, pressed: true, .. })), "{fed:?}");
    assert!(fed.iter().any(|e| matches!(e, egui::Event::Key { key: egui::Key::B, pressed: false, .. })), "{fed:?}");

    let text = send(&tx, "ui.type", json!({"text": "x"}));
    let (reply, fed) = drive(&mut h, &text, 2);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(fed, vec![egui::Event::Text("x".into())]);
}

#[test]
fn a_pointer_request_is_answered_by_the_pass_that_reads_it() {
    let (mut h, tx) = app();
    let reply = send(&tx, "ui.pointer", json!({"events": [{"kind": "move", "x": 40, "y": 40}]}));
    pass(&mut h);
    let reply = reply.try_recv().expect("ui.pointer does not wait for another pass");
    assert_eq!(reply["ok"], true, "{reply}");
}

/// A rejected request queues nothing and is answered at once, so the bridge never waits on it.
#[test]
fn a_rejected_move_is_answered_by_the_first_pass() {
    let (mut h, tx) = app();
    let reply = send(&tx, "ui.move", json!({"x": 1, "y": 1, "bogus": true}));
    pass(&mut h);
    let reply = reply.try_recv().expect("answered by the first pass");
    assert_eq!(reply["ok"], false, "{reply}");
    assert!(h.state_mut().take_synthetic_step().is_empty());
}
