//! The toolbar's third mode (`Panels.toolbar_floating`, `panels::toolbar_float`): a palette over
//! the canvas with a grip and a pin, instead of a column beside it.

use egui::{Pos2, Rect, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use photocraft_ui_egui::PhotocraftApp;
use photocraft_ui_egui::control::{ControlRequest, Outcome, handle};
use photocraft_ui_egui::panels::toolbar_float::{DEFAULT_OFFSET, GRIP_LABEL, PIN_LABEL, Place, clamp_offset};
use photocraft_ui_egui::state::Tool;
use photocraft_ui_egui::theme::ThemeKind;
use serde_json::{Value, json};

type Harness = egui_kittest::Harness<'static, PhotocraftApp>;

/// The canvas area the toolbar-only harness pretends to have.
const CANVAS: Rect = Rect { min: pos2(100.0, 40.0), max: pos2(880.0, 1380.0) };

fn call(app: &mut PhotocraftApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (req, _rx) = ControlRequest::new(method, params);
    match handle(app, ctx, &req) {
        Outcome::Done(reply) => reply,
        _ => panic!("{method} did not answer at once"),
    }
}

fn new_app() -> PhotocraftApp {
    PhotocraftApp::new(photocraft_engine::Session::new(), Default::default())
}

/// Only the toolbar, as upstream's toolbar tests draw it; what is left of the frame beside it is
/// kept under `"left"`.
fn toolbar_harness(kind: ThemeKind, floating: bool) -> Harness {
    columns_harness(kind, floating, false)
}

fn columns_harness(kind: ThemeKind, floating: bool, double: bool) -> Harness {
    let mut app = new_app();
    app.ui.tool = Tool::Move;
    app.ui.panels.toolbar_floating = floating;
    app.ui.panels.toolbar_double = double;
    app.last_canvas_rect = CANVAS;
    let mut h = egui_kittest::Harness::builder().with_size(vec2(900.0, 1400.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut PhotocraftApp| {
            if ui.ctx().fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                photocraft_ui_egui::panels::toolbar(app, ui);
                let left = ui.available_rect_before_wrap().left();
                ui.data_mut(|d| d.insert_temp(egui::Id::new("left"), left));
            }
        },
        app,
    );
    PhotocraftApp::setup_context(&h.ctx, kind);
    h.run_steps(3);
    h
}

/// Where the toolbar-only harness's frame begins: the margin of the test's own root ui.
const EDGE: f32 = 8.0;

fn left(h: &Harness) -> f32 {
    h.ctx.data(|d| d.get_temp::<f32>(egui::Id::new("left"))).unwrap_or(f32::NAN)
}

/// The tool buttons the last pass drew, in slot order: tool-sized and clickable.
fn tool_buttons(h: &Harness, side: f32) -> Vec<Rect> {
    h.ctx.viewport(|v| {
        v.prev_pass.widgets.layers().flat_map(|(_, w)| w.iter()).filter(|w| w.rect.size() == egui::Vec2::splat(side) && w.sense.senses_click()).map(|w| w.rect).collect()
    })
}

fn click(h: &mut Harness, p: Pos2) {
    h.hover_at(p);
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
        h.run_steps(1);
    }
}

/// Presses at `from`, moves by `by` in small steps and lets go.
fn drag(h: &mut Harness, from: Pos2, by: egui::Vec2) {
    h.hover_at(from);
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
    h.run_steps(1);
    for i in 1..=10 {
        h.event(egui::Event::PointerMoved(from + by * (i as f32 / 10.0)));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton { pos: from + by, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
    h.run_steps(2);
}

fn grip(h: &Harness) -> Rect {
    h.get_by_label(GRIP_LABEL).rect()
}

fn place(h: &Harness) -> Place {
    h.state().ui.panels.toolbar_float
}

#[test]
fn floating_mode_round_trips_through_ui_set_and_ui_inspect() {
    let mut app = new_app();
    let ctx = egui::Context::default();
    let panels = |app: &mut PhotocraftApp| call(app, &ctx, "ui.inspect", json!({}))["result"]["panels"].clone();
    assert_eq!(panels(&mut app)["toolbar_floating"], false, "docked by default");
    assert_eq!(panels(&mut app)["toolbar_float"], json!({"pos": null, "pinned": false}));

    let reply = call(&mut app, &ctx, "ui.set", json!({"panels": {"toolbar_floating": true}}));
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(app.ui.panels.toolbar_floating);
    assert!(app.ui.panels.toolbar, "floating is a mode of the toolbar, not a way to hide it");
    assert_eq!(panels(&mut app)["toolbar_floating"], true);

    let reply = call(&mut app, &ctx, "ui.set", json!({"panels": {"toolbar_float": {"pos": [40.0, 64.0], "pinned": true}}}));
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(app.ui.panels.toolbar_float, Place { pos: Some([40.0, 64.0]), pinned: true });
    assert_eq!(panels(&mut app)["toolbar_float"], json!({"pos": [40.0, 64.0], "pinned": true}));

    let reply = call(&mut app, &ctx, "ui.set", json!({"panels": {"toolbar_float": {"nope": 1}}}));
    assert_eq!(reply["ok"], false, "an unknown field of the place is refused: {reply}");
    assert_eq!(app.ui.panels.toolbar_float, Place { pos: Some([40.0, 64.0]), pinned: true }, "and changes nothing");

    let reply = call(&mut app, &ctx, "ui.set", json!({"panels": {"toolbar_floating": false}}));
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(panels(&mut app)["toolbar_floating"], false);
    assert_eq!(panels(&mut app)["toolbar_float"]["pinned"], true, "the place is kept for the next time it floats");
}

/// A workspace saved before the mode existed still loads, docked.
#[test]
fn panels_saved_without_the_mode_load_docked() {
    let mut saved = serde_json::to_value(photocraft_ui_egui::state::Panels::default()).unwrap();
    let fields = saved.as_object_mut().unwrap();
    assert!(fields.remove("toolbar_floating").is_some() && fields.remove("toolbar_float").is_some());
    let panels: photocraft_ui_egui::state::Panels = serde_json::from_value(saved).unwrap();
    assert!(!panels.toolbar_floating);
    assert_eq!(panels.toolbar_float, Place::default());
}

/// The whole app: the column the docked toolbar takes goes to the canvas when it floats, and
/// comes back when it docks.
#[test]
fn floating_toolbar_gives_the_canvas_the_freed_width() {
    let mut h: Harness = egui_kittest::Harness::builder().with_size(vec2(1400.0, 900.0)).with_max_steps(64).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, ThemeKind::Studio);
        let mut app = new_app();
        app.run("file.new", json!({"width": 64, "height": 64})).unwrap();
        app.set_theme(&cc.egui_ctx, ThemeKind::Studio);
        app
    });
    h.run_steps(8);
    let docked = h.state().last_canvas_rect;
    assert!(docked.left() >= 40.0, "the docked toolbar takes a column: {docked:?}");
    assert!(h.query_by_label(GRIP_LABEL).is_none(), "no grip while docked");

    let ctx = h.ctx.clone();
    assert_eq!(call(h.state_mut(), &ctx, "ui.set", json!({"panels": {"toolbar_floating": true}}))["ok"], true);
    h.run_steps(6);
    let floating = h.state().last_canvas_rect;
    assert_eq!(floating.left(), 0.0, "the canvas starts at the window's edge: {floating:?}");
    assert_eq!(floating.width(), docked.width() + docked.left(), "the canvas got the whole column: {docked:?} -> {floating:?}");
    assert_eq!((floating.right(), floating.top(), floating.bottom()), (docked.right(), docked.top(), docked.bottom()), "nothing else moved");
    let palette = grip(&h);
    assert!(floating.contains_rect(palette), "the palette is drawn over the canvas: {palette:?} in {floating:?}");
    let corner = palette.min - floating.min;
    assert!((DEFAULT_OFFSET[0]..DEFAULT_OFFSET[0] + 12.0).contains(&corner.x), "at its default corner, measured from the full-width canvas: {corner:?}");
    assert!((DEFAULT_OFFSET[1]..DEFAULT_OFFSET[1] + 12.0).contains(&corner.y), "{corner:?}");

    assert_eq!(call(h.state_mut(), &ctx, "ui.set", json!({"panels": {"toolbar_floating": false}}))["ok"], true);
    h.run_steps(6);
    assert_eq!(h.state().last_canvas_rect, docked, "docking gives the column back");
    assert!(h.query_by_label(GRIP_LABEL).is_none());
}

#[test]
fn a_tool_click_in_floating_mode_selects_the_tool() {
    for (kind, side) in [(ThemeKind::Studio, 36.0), (ThemeKind::Pro, 30.0)] {
        let docked = toolbar_harness(kind, false);
        let docked_buttons = tool_buttons(&docked, side);
        assert!(left(&docked) >= EDGE + 40.0, "{kind:?}: docked, the toolbar is a column of the frame");

        let mut h = toolbar_harness(kind, true);
        assert_eq!(left(&h), EDGE, "{kind:?}: floating, the toolbar takes nothing from the frame");
        let buttons = tool_buttons(&h, side);
        assert_eq!(buttons.len(), docked_buttons.len(), "{kind:?}: the palette draws the same tool buttons as the column");
        assert!(buttons.iter().all(|b| CANVAS.contains_rect(*b)), "{kind:?}: every tool button is over the canvas: {buttons:?}");
        assert_eq!(h.state().ui.tool, Tool::Move);
        // The second slot is the marquee group; with no tool of it used yet it shows its first.
        click(&mut h, buttons[1].center());
        assert_eq!(h.state().ui.tool, Tool::RectMarquee, "{kind:?}: a click on the palette picks the tool");
        click(&mut h, buttons[0].center());
        assert_eq!(h.state().ui.tool, Tool::Move, "{kind:?}");
        assert!(h.state().ui.panels.toolbar_floating, "{kind:?}: picking tools leaves the mode alone");
        assert_eq!(place(&h), Place::default(), "{kind:?}: and the place");
    }
}

#[test]
fn the_grip_drags_the_palette_and_keeps_it_inside_the_canvas() {
    let mut h = toolbar_harness(ThemeKind::Studio, true);
    let start = grip(&h);
    assert!(start.left() >= CANVAS.left() + DEFAULT_OFFSET[0] && start.top() >= CANVAS.top() + DEFAULT_OFFSET[1], "{start:?}");
    assert_eq!(place(&h).pos, None, "untouched until dragged");

    drag(&mut h, start.center(), vec2(200.0, 80.0));
    let moved = grip(&h);
    let by = moved.min - start.min;
    // The first few points of a press are a click until they turn into a drag.
    assert!((190.0..=200.0).contains(&by.x) && (72.0..=80.0).contains(&by.y), "the palette followed the grip: {by:?}");
    let pos = place(&h).pos.expect("the place is remembered in the panels state");
    assert!((pos[0] - (DEFAULT_OFFSET[0] + by.x)).abs() < 0.01 && (pos[1] - (DEFAULT_OFFSET[1] + by.y)).abs() < 0.01, "{pos:?} after {by:?}");

    // Far past the canvas corner: it stops inside.
    drag(&mut h, moved.center(), vec2(3000.0, 3000.0));
    let side = tool_buttons(&h, 36.0);
    assert!(!side.is_empty() && side.iter().all(|b| CANVAS.contains_rect(*b)), "the tools stay over the canvas: {side:?}");
    let pos = place(&h).pos.unwrap();
    assert_eq!(clamp_offset(CANVAS, egui::Vec2::ZERO, pos.into()), egui::Vec2::from(pos), "the remembered place is inside the canvas area");
    assert!(CANVAS.contains_rect(grip(&h)));

    // And past the other corner.
    let at = grip(&h).center();
    drag(&mut h, at, vec2(-3000.0, -3000.0));
    let pos = place(&h).pos.unwrap();
    assert!(pos[0] > 0.0 && pos[0] <= DEFAULT_OFFSET[0] && pos[1] > 0.0 && pos[1] <= DEFAULT_OFFSET[1], "{pos:?}");

    // A smaller canvas area (the window shrank, a dock opened): the palette moves back inside
    // without the remembered place being rewritten.
    let at = grip(&h).center();
    drag(&mut h, at, vec2(600.0, 0.0));
    let remembered = place(&h);
    h.state_mut().last_canvas_rect = Rect::from_min_max(CANVAS.min, pos2(400.0, CANVAS.bottom()));
    h.run_steps(3);
    assert!(grip(&h).right() <= 400.0, "{:?}", grip(&h));
    assert_eq!(place(&h), remembered);
}

#[test]
fn a_pinned_palette_does_not_move() {
    let mut h = toolbar_harness(ThemeKind::Studio, true);
    h.get_by_label(PIN_LABEL).click();
    h.run_steps(2);
    assert!(place(&h).pinned, "the pin button pins");
    let start = grip(&h);
    drag(&mut h, start.center(), vec2(200.0, 80.0));
    assert_eq!(grip(&h), start, "the grip of a pinned palette moves nothing");
    assert_eq!(place(&h), Place { pos: None, pinned: true });

    h.get_by_label(PIN_LABEL).click();
    h.run_steps(2);
    assert!(!place(&h).pinned, "the pin button lets go again");
    drag(&mut h, start.center(), vec2(200.0, 80.0));
    assert!(grip(&h).left() > start.left() + 150.0);
}

/// Pro themes have the header chevron: a click still switches one and two columns (upstream's
/// behaviour, #1197), its context menu offers the four states.
#[test]
fn the_header_chevron_menu_switches_the_four_states() {
    let mut h = toolbar_harness(ThemeKind::Pro, false);
    let mode = |h: &Harness| (h.state().ui.panels.toolbar_floating, h.state().ui.panels.toolbar_double);
    let choose = |h: &mut Harness, label: &str| {
        h.get_by_label("Toolbar").click_secondary();
        h.run_steps(3);
        h.get_by_label(label).click();
        h.run_steps(3);
    };
    choose(&mut h, "Docked, two columns");
    assert_eq!(mode(&h), (false, true));
    choose(&mut h, "Floating, two columns");
    assert_eq!(mode(&h), (true, true));
    assert_eq!(left(&h), EDGE);
    h.get_by_label("Toolbar").click();
    h.run_steps(2);
    assert_eq!(mode(&h), (true, false), "the chevron's click still switches the columns, floating or not");
    choose(&mut h, "Docked, one column");
    assert_eq!(mode(&h), (false, false));
    assert!(left(&h) >= EDGE + 40.0);
}

/// The grip's context menu docks the palette: the way back without a chevron (Studio has none).
#[test]
fn the_grip_menu_docks_the_palette() {
    let mut h = toolbar_harness(ThemeKind::Studio, true);
    h.get_by_label(GRIP_LABEL).click_secondary();
    h.run_steps(3);
    h.get_by_label("Docked, one column").click();
    h.run_steps(3);
    assert!(!h.state().ui.panels.toolbar_floating);
    assert!(left(&h) >= EDGE + 40.0);
    assert!(h.query_by_label(GRIP_LABEL).is_none());
}

/// Left edges (rounded) of the tool buttons: one value per column.
fn columns(buttons: &[Rect]) -> Vec<i32> {
    let mut xs: Vec<i32> = buttons.iter().map(|b| b.left().round() as i32).collect();
    xs.sort_unstable();
    xs.dedup();
    xs
}

/// Height the tool buttons span.
fn span(buttons: &[Rect]) -> f32 {
    let top = buttons.iter().map(|b| b.top()).fold(f32::MAX, f32::min);
    let bottom = buttons.iter().map(|b| b.bottom()).fold(f32::MIN, f32::max);
    bottom - top
}

/// Button positions relative to the first, so a docked and a floating layout compare.
fn relative(buttons: &[Rect]) -> Vec<(i32, i32)> {
    buttons.iter().map(|b| ((b.min.x - buttons[0].min.x).round() as i32, (b.min.y - buttons[0].min.y).round() as i32)).collect()
}

fn palette_area(h: &Harness) -> Rect {
    h.ctx.memory(|m| m.area_rect(egui::Id::new("toolbar-float"))).expect("the palette is drawn")
}

/// With `toolbar_double` on, the floating palette is two tools wide and shorter than the one-column
/// palette, with the same tools in the same order and the same dividers as the docked two columns.
#[test]
fn floating_honours_toolbar_double() {
    for (kind, side) in [(ThemeKind::Studio, 36.0), (ThemeKind::Pro, 30.0)] {
        let single = toolbar_harness(kind, true);
        let double = columns_harness(kind, true, true);
        let docked_double = columns_harness(kind, false, true);
        let one = tool_buttons(&single, side);
        let two = tool_buttons(&double, side);
        assert_eq!(columns(&one).len(), 1, "{kind:?}: one column floating");
        assert_eq!(columns(&two).len(), 2, "{kind:?}: two tools wide: {two:?}");
        assert!(span(&two) < span(&one) * 0.75, "{kind:?}: shorter: {} vs {}", span(&two), span(&one));
        assert_eq!(two.len(), one.len(), "{kind:?}: every tool of the one-column palette is present");
        assert_eq!(relative(&two), relative(&tool_buttons(&docked_double, side)), "{kind:?}: the docked two-column layout, dividers included");
        assert!(palette_area(&double).width() > palette_area(&single).width(), "{kind:?}: the frame is wider");
        assert!(palette_area(&double).height() < palette_area(&single).height(), "{kind:?}: and shorter");
        assert!(CANVAS.contains_rect(palette_area(&double)) && CANVAS.contains_rect(palette_area(&single)), "{kind:?}");
    }
}

#[test]
fn a_tool_click_in_the_two_column_palette_selects_the_tool() {
    for (kind, side) in [(ThemeKind::Studio, 36.0), (ThemeKind::Pro, 30.0)] {
        let mut h = columns_harness(kind, true, true);
        let buttons = tool_buttons(&h, side);
        assert!(buttons.iter().all(|b| CANVAS.contains_rect(*b)), "{kind:?}: {buttons:?}");
        click(&mut h, buttons[1].center());
        assert_eq!(h.state().ui.tool, Tool::RectMarquee, "{kind:?}");
        click(&mut h, buttons[0].center());
        assert_eq!(h.state().ui.tool, Tool::Move, "{kind:?}");
        assert!(h.state().ui.panels.toolbar_floating && h.state().ui.panels.toolbar_double, "{kind:?}: the modes are left alone");
    }
}

#[test]
fn switching_columns_while_floating_keeps_the_top_left_corner() {
    let mut h = columns_harness(ThemeKind::Studio, true, false);
    let start = grip(&h);
    drag(&mut h, start.center(), vec2(120.0, 60.0));
    let before = palette_area(&h).min;
    let pos = place(&h);
    let ctx = h.ctx.clone();
    for double in [true, false, true] {
        assert_eq!(call(h.state_mut(), &ctx, "ui.set", json!({"panels": {"toolbar_double": double}}))["ok"], true);
        h.run_steps(4);
        assert_eq!(palette_area(&h).min, before, "double={double}: the top-left corner stays");
        assert_eq!(place(&h), pos, "double={double}: and the remembered place");
    }
    // Near the bottom-right corner the wider palette is clamped, not pushed out of the canvas.
    let at = grip(&h).center();
    drag(&mut h, at, vec2(3000.0, 3000.0));
    for double in [false, true] {
        assert_eq!(call(h.state_mut(), &ctx, "ui.set", json!({"panels": {"toolbar_double": double}}))["ok"], true);
        h.run_steps(4);
        assert!(CANVAS.contains_rect(palette_area(&h)), "double={double}: {:?}", palette_area(&h));
    }
}

#[test]
fn the_palette_stays_inside_a_small_canvas_in_both_forms() {
    let small = Rect::from_min_max(pos2(100.0, 40.0), pos2(300.0, 400.0));
    for double in [false, true] {
        let mut h = columns_harness(ThemeKind::Studio, true, double);
        h.state_mut().last_canvas_rect = small;
        h.run_steps(4);
        assert!(small.contains_rect(palette_area(&h)), "double={double}: {:?} in {small:?}", palette_area(&h));
        let at = grip(&h).center();
        drag(&mut h, at, vec2(3000.0, 3000.0));
        assert!(small.contains_rect(palette_area(&h)), "double={double} dragged: {:?}", palette_area(&h));
        let at = grip(&h).center();
        drag(&mut h, at, vec2(-3000.0, -3000.0));
        assert!(small.contains_rect(palette_area(&h)), "double={double} dragged back: {:?}", palette_area(&h));
    }
}

const FOUR: [&str; 4] = ["Docked, one column", "Docked, two columns", "Floating, one column", "Floating, two columns"];

/// The grip's menu (Studio has no chevron) lists the four states and each sets the right pair.
#[test]
fn the_grip_menu_offers_the_four_states() {
    for (i, label) in FOUR.iter().enumerate() {
        let (want_floating, want_double) = (i >= 2, i % 2 == 1);
        for (start_floating, start_double) in [(true, false), (true, true)] {
            let mut h = columns_harness(ThemeKind::Studio, start_floating, start_double);
            h.get_by_label(GRIP_LABEL).click_secondary();
            h.run_steps(3);
            for other in FOUR {
                assert!(h.query_by_label(other).is_some(), "the grip menu lists {other}");
            }
            h.get_by_label(label).click();
            h.run_steps(4);
            let p = &h.state().ui.panels;
            assert_eq!((p.toolbar_floating, p.toolbar_double), (want_floating, want_double), "{label} from {start_floating},{start_double}");
        }
    }
}

/// The header chevron's menu (Pro) lists the four states and each sets the right pair.
#[test]
fn the_chevron_menu_offers_the_four_states() {
    for (i, label) in FOUR.iter().enumerate() {
        let (want_floating, want_double) = (i >= 2, i % 2 == 1);
        for (start_floating, start_double) in [(false, false), (true, true)] {
            let mut h = columns_harness(ThemeKind::Pro, start_floating, start_double);
            h.get_by_label("Toolbar").click_secondary();
            h.run_steps(3);
            for other in FOUR {
                assert!(h.query_by_label(other).is_some(), "the chevron menu lists {other}");
            }
            h.get_by_label(label).click();
            h.run_steps(4);
            let p = &h.state().ui.panels;
            assert_eq!((p.toolbar_floating, p.toolbar_double), (want_floating, want_double), "{label} from {start_floating},{start_double}");
        }
    }
}
