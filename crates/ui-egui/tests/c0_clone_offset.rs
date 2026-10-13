//! Where the Clone Stamp and the Healing Brush deposit and where they sample, measured through
//! the real app with real pointer events.
//!
//! A user's screen recording suggested the stamp clones from, or paints to, a place offset from
//! the centre of the brush circle. The circle is drawn round the pointer (`canvas.rs`,
//! `tool_cursor::circle(&painter, p, r, ..)`), so the claim to test is: a dab painted with the
//! pointer at P is centred on P, and its pixels are the ones centred on the source point S.
//!
//! The Clone Stamp copies whole pixels (the offset is rounded once to a pixel), so half a
//! document pixel per axis is the most either centre may be off; anything beyond that is an
//! offset bug. Every pixel of the test document has its own colour, so a cloned pixel names the
//! pixel it came from.
//!
//! Measured on upstream a0ff3a01 (90 cases: 1x and 2x displays, zoom 50%, 100% and 400%, brush
//! diameters 5, 9 and 10, five sub-pixel pointer positions): the dab was centred on the pointer to
//! 0.01 px in every case, and the copied patch was up to 0.8 px off the source point. The source
//! point was rounded to a whole pixel when it was set (`retouch_ui::set_source`) and the offset
//! rounded again when the stroke began, so the two errors added up: with the pointer inside pixel
//! 100 for the source and at the centre of pixel 139 for the dab, pixel 139 got pixel 99. The fix
//! keeps the source point as clicked, which leaves the one rounding; these tests hold it there.

use std::sync::Arc;

use egui::{Modifiers, PointerButton, Pos2, vec2};
use photocraft_geom::Rect;
use photocraft_ui_egui::PhotocraftApp;
use photocraft_ui_egui::canvas::ViewXform;
use photocraft_ui_egui::state::Tool;
use serde_json::json;

type Harness = egui_kittest::Harness<'static, PhotocraftApp>;

const W: i32 = 240;
const H: i32 = 160;
/// The grey the Healing Brush document is filled with.
const FLAT: f32 = 128.0 / 255.0;

/// Every pixel its own colour: red is the column, green the row.
fn ramp(x: i32, y: i32) -> [f32; 4] {
    [x as f32 / 255.0, y as f32 / 255.0, 0.25, 1.0]
}

fn write(app: &mut PhotocraftApp, px: &dyn Fn(i32, i32) -> [f32; 4]) {
    let st = app.session.active_mut().unwrap();
    let doc = Arc::make_mut(&mut st.doc);
    let surf = doc.layers[0].surface_mut().unwrap();
    let fmt = surf.format();
    let mut data = Vec::new();
    for y in 0..H {
        for x in 0..W {
            data.extend(photocraft_raster::from_rgba(&fmt, px(x, y)));
        }
    }
    surf.write_region(Rect::new(0, 0, W, H), &data);
}

fn read(app: &PhotocraftApp, x: i32, y: i32) -> [f32; 4] {
    app.session.active().unwrap().doc.layers[0].surface().unwrap().rgba(x, y)
}

/// The whole app with a `W` × `H` one-layer document, on a display with `ppp` pixels per point.
fn harness(ppp: f32) -> Harness {
    let mut h: Harness = egui_kittest::Harness::builder().with_size(vec2(1400.0, 900.0)).with_pixels_per_point(ppp).with_max_steps(64).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width": W, "height": H, "background": "white"})).unwrap();
        app
    });
    h.run_steps(6);
    h
}

/// Selects `tool` with a hard round brush of `size` pixels and shows the document at `zoom`
/// (1 = one document pixel per display pixel), centred.
fn set_up(h: &mut Harness, tool: Tool, size: u32, zoom: f32) {
    h.state_mut().ui.tool = tool;
    h.run_steps(2);
    h.state_mut().run("tools.setBrush", json!({"brush": {"size": size, "hardness": 1.0}})).unwrap();
    let view = &mut h.state_mut().ui.views[0];
    view.zoom = zoom;
    view.center = [W as f32 / 2.0, H as f32 / 2.0];
    view.fit_pending = false;
    h.run_steps(3);
    assert_eq!(h.state().ui.views[0].zoom, zoom, "the view kept the zoom");
    assert_eq!(h.state().session.tools.brush.size, size as f32, "the tool paints with the brush just set");
}

/// The screen position of a document point, by the transform the canvas itself draws with.
fn screen(h: &Harness, doc: [f64; 2]) -> Pos2 {
    ViewXform::active(h.state()).expect("a document is shown").to_screen(doc[0] as f32, doc[1] as f32)
}

/// A click at a document point with `mods` held from before the press until after the release.
/// Returns the document point the canvas maps the pointer to (f32 screen coordinates in between).
fn click(h: &mut Harness, doc: [f64; 2], mods: Modifiers) -> [f64; 2] {
    let p = screen(h, doc);
    let seen = ViewXform::active(h.state()).unwrap().to_doc(p);
    h.event(egui::Event::ModifiersChanged(mods));
    h.run_steps(1);
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: mods });
        h.run_steps(1);
    }
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(1);
    seen
}

/// What one Clone Stamp dab did to the ramp document.
#[derive(Debug)]
struct Dab {
    /// Centre of the painted pixels, weighted by how much of each was painted.
    centre: [f64; 2],
    /// Source pixel minus destination pixel, the same for every fully painted pixel.
    offset: [i32; 2],
    /// Fully painted pixels.
    solid: usize,
}

fn measure_dab(app: &PhotocraftApp) -> Dab {
    let byte = |v: f32| (v * 255.0).round() as i32;
    // A fully painted pixel holds another pixel's colour exactly, which names its offset.
    let mut votes = std::collections::HashMap::<[i32; 2], usize>::new();
    let mut changed = Vec::new();
    for y in 0..H {
        for x in 0..W {
            let px = read(app, x, y);
            if (byte(px[0]), byte(px[1])) != (x, y) {
                changed.push((x, y, px));
                *votes.entry([byte(px[0]) - x, byte(px[1]) - y]).or_default() += 1;
            }
        }
    }
    assert!(!changed.is_empty(), "the click painted nothing");
    // A partly painted edge pixel reads as a shorter offset in the same direction, so the longest
    // one is the real one.
    let (&offset, &solid) = votes.iter().max_by_key(|(offset, _)| offset[0].abs() + offset[1].abs()).unwrap();
    // A partly covered edge pixel is a mix of its own colour and its source's: the share of the
    // source is its coverage.
    let (mut sum, mut cx, mut cy) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y, px) in &changed {
        let axis = if offset[0] != 0 { 0 } else { 1 };
        let (own, source) = if axis == 0 { (*x, *x + offset[0]) } else { (*y, *y + offset[1]) };
        let cover = (f64::from(px[axis]) * 255.0 - f64::from(own)) / f64::from(source - own);
        assert!((-0.05..=1.05).contains(&cover), "pixel {x},{y} is not a mix of itself and its source at {offset:?}: {px:?}");
        let cover = cover.clamp(0.0, 1.0);
        sum += cover;
        cx += cover * (f64::from(*x) + 0.5);
        cy += cover * (f64::from(*y) + 0.5);
    }
    Dab { centre: [cx / sum, cy / sum], offset, solid }
}

/// Half a pixel, the most one rounding to whole pixels can be off, and a little for the measuring
/// (a dab's centre is worked out from 8-bit edge pixels).
const HALF: f64 = 0.52;

/// The sub-pixel places a pointer can be in a pixel: its centre, a corner, and off-centre.
const FRACTIONS: [[f64; 2]; 5] = [[0.5, 0.5], [0.0, 0.0], [0.3, 0.7], [0.8, 0.2], [0.49, 0.51]];

/// Clone Stamp: the dab is centred on the pointer and its pixels come from round the source point,
/// for odd and even brush diameters, on plain and scaled displays, zoomed in and out.
#[test]
fn clone_stamp_deposits_at_the_pointer_and_samples_at_the_source() {
    let (mut worst_centre, mut worst_source) = (0.0f64, 0.0f64);
    let mut cases = 0;
    for ppp in [1.0f32, 2.0] {
        let mut h = harness(ppp);
        for zoom in [1.0f32, 4.0, 0.5] {
            for size in [9u32, 10, 5] {
                set_up(&mut h, Tool::CloneStamp, size, zoom);
                for (i, fs) in FRACTIONS.iter().enumerate() {
                    let fp = FRACTIONS[(i * 2 + 1) % FRACTIONS.len()];
                    write(h.state_mut(), &ramp);
                    let s = click(&mut h, [100.0 + fs[0], 70.0 + fs[1]], Modifiers::ALT);
                    assert!(h.state().session.presets.clone.active().source.is_some(), "⌥-click set the source");
                    let p = click(&mut h, [139.0 + fp[0], 91.0 + fp[1]], Modifiers::NONE);
                    let dab = measure_dab(h.state());
                    let centre = [dab.centre[0] - p[0], dab.centre[1] - p[1]];
                    // The patch the dab copied is centred here; it should be S.
                    let source = [dab.centre[0] + f64::from(dab.offset[0]) - s[0], dab.centre[1] + f64::from(dab.offset[1]) - s[1]];
                    let label = format!("ppp {ppp} zoom {zoom} size {size} S {s:?} P {p:?}: {dab:?}");
                    eprintln!("{label}  centre-P {centre:.3?}  source-S {source:.3?}");
                    assert!(dab.solid >= 4, "{label}");
                    assert!(centre[0].abs() <= 0.05 && centre[1].abs() <= 0.05, "the dab is not centred on the pointer, off by {centre:?}: {label}");
                    assert!(source[0].abs() <= HALF && source[1].abs() <= HALF, "the dab was not copied from round the source point, off by {source:?}: {label}");
                    worst_centre = worst_centre.max(centre[0].abs()).max(centre[1].abs());
                    worst_source = worst_source.max(source[0].abs()).max(source[1].abs());
                    cases += 1;
                }
            }
        }
    }
    eprintln!("clone stamp, {cases} cases: dab centre off the pointer by at most {worst_centre:.3} px, source patch off the source point by at most {worst_source:.3} px");
}

/// The case a user sees when zoomed in: the pointer anywhere inside one pixel for the source, then
/// at the same place inside another pixel for the dab. The pixel under the pointer must become the
/// pixel that was under the pointer, wherever in the pixel that place is.
#[test]
fn the_pixel_under_the_pointer_is_cloned_to_the_pixel_under_the_pointer() {
    let mut h = harness(1.0);
    set_up(&mut h, Tool::CloneStamp, 5, 16.0);
    for f in [0.05, 0.25, 0.45, 0.5, 0.55, 0.75, 0.95] {
        // Source to the left of and above the dab, then to the right of and below it.
        for (s, p) in [([100.0, 70.0], [112.0, 81.0]), ([112.0, 81.0], [100.0, 70.0])] {
            write(h.state_mut(), &ramp);
            click(&mut h, [s[0] + f, s[1] + f], Modifiers::ALT);
            click(&mut h, [p[0] + f, p[1] + f], Modifiers::NONE);
            let got = read(h.state(), p[0] as i32, p[1] as i32);
            let from = [f64::from((got[0] * 255.0).round()), f64::from((got[1] * 255.0).round())];
            assert_eq!(from, s, "pointer {f} into the pixel: pixel {p:?} was painted with pixel {from:?}, not the source pixel {s:?}");
        }
    }
}

/// Aligned (the default): a second stroke elsewhere keeps the first one's offset, so it samples
/// as far from the source point as it paints from the first stroke.
#[test]
fn an_aligned_second_dab_keeps_the_offset() {
    let mut h = harness(1.0);
    set_up(&mut h, Tool::CloneStamp, 9, 2.0);
    write(h.state_mut(), &ramp);
    click(&mut h, [100.5, 70.5], Modifiers::ALT);
    click(&mut h, [139.5, 91.5], Modifiers::NONE);
    let first = measure_dab(h.state());
    assert_eq!(first.offset, [-39, -21]);
    write(h.state_mut(), &ramp);
    let p = click(&mut h, [150.5, 80.5], Modifiers::NONE);
    let second = measure_dab(h.state());
    assert_eq!(second.offset, first.offset, "{second:?}");
    assert!((second.centre[0] - p[0]).abs() <= 0.5 && (second.centre[1] - p[1]).abs() <= 0.5, "{second:?} at {p:?}");
}

/// Healing Brush: it blends the source's texture into the surroundings rather than copying
/// pixels, so the measure is a feature. A white 3 × 3 dot centred on the source point, healed
/// onto the same flat grey, must come out centred on the pointer.
#[test]
fn healing_brush_brings_the_source_point_to_the_pointer() {
    let dot = |x: i32, y: i32| if (99..102).contains(&x) && (69..72).contains(&y) { [1.0, 1.0, 1.0, 1.0] } else { [FLAT, FLAT, FLAT, 1.0] };
    let mut worst = 0.0f64;
    for ppp in [1.0f32, 2.0] {
        let mut h = harness(ppp);
        for zoom in [1.0f32, 4.0] {
            for size in [13u32, 14] {
                set_up(&mut h, Tool::Healing, size, zoom);
                for fp in [[0.5, 0.5], [0.0, 0.0], [0.3, 0.7]] {
                    write(h.state_mut(), &dot);
                    // The dot's centre.
                    let s = click(&mut h, [100.5, 70.5], Modifiers::ALT);
                    let p = click(&mut h, [139.0 + fp[0], 91.0 + fp[1]], Modifiers::NONE);
                    let (mut sum, mut cx, mut cy) = (0.0f64, 0.0f64, 0.0f64);
                    for y in 0..H {
                        for x in 0..W {
                            // The source dot itself stays where it is; look at the rest.
                            if (95..106).contains(&x) && (65..76).contains(&y) {
                                continue;
                            }
                            let lift = f64::from(read(h.state(), x, y)[0] - FLAT).max(0.0);
                            if lift > 0.02 {
                                sum += lift;
                                cx += lift * (f64::from(x) + 0.5);
                                cy += lift * (f64::from(y) + 0.5);
                            }
                        }
                    }
                    let label = format!("ppp {ppp} zoom {zoom} size {size} S {s:?} P {p:?}");
                    assert!(sum > 1.0, "the healed dab shows no dot (lift {sum}): {label}");
                    let off = [cx / sum - p[0], cy / sum - p[1]];
                    eprintln!("{label}: healed dot centre - P {off:.3?} (lift {sum:.2})");
                    assert!(off[0].abs() <= HALF && off[1].abs() <= HALF, "the healed dot is not at the pointer, off by {off:?}: {label}");
                    worst = worst.max(off[0].abs()).max(off[1].abs());
                }
            }
        }
    }
    eprintln!("healing brush: healed dot centre off the pointer by at most {worst:.3} px");
}
