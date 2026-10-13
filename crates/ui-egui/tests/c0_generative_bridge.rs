//! The `c0.*` requests the C0VIBE host's image-generation panel sends (`src/c0_generative.rs`):
//! capture, selection mask, place, open from bytes, and dismissing a pending file dialog.
//!
//! They run through `control::handle` exactly as the page bridge's requests do, in a native app. What
//! the browser adds (the page function, base64 over `window.__photocraftControl`, the real file
//! picker) is not covered here.

use std::cell::RefCell;
use std::rc::Rc;

use base64::Engine as _;
use photocraft_codecs::{ChannelLayout, Format, Image, SampleType as CodecSample};
use photocraft_color::{ColorMode, PixelFormat, SampleType};
use photocraft_doc::{Document, Layer, LayerContent};
use photocraft_geom::{Rect, Size};
use photocraft_raster::Surface;
use photocraft_ui_egui::control::{ControlRequest, Outcome, handle};
use photocraft_ui_egui::{FileDialogRequest, PhotocraftApp, Services};
use serde_json::{Value, json};

fn services() -> Services {
    Services {
        import: Some(Box::new(|name: &str, bytes: &[u8], depth: usize| {
            photocraft_io::import_with_svg_group_depth(name, bytes, depth).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
        })),
        ..Default::default()
    }
}

/// The colour of pixel (x, y) of the 4×4 pattern: every pixel different, all opaque.
fn pattern(x: u32, y: u32) -> [u8; 4] {
    [(x * 60 + 10) as u8, (y * 60 + 20) as u8, ((x + y) * 30 + 5) as u8, 255]
}

fn pattern_document() -> Document {
    let mut doc = Document::new("pattern", Size::new(4, 4), ColorMode::Rgb, SampleType::U8);
    let bytes: Vec<u8> = (0..4).flat_map(|y| (0..4).flat_map(move |x| pattern(x, y))).collect();
    let surface = Surface::from_interleaved(PixelFormat::RGBA8, Rect::new(0, 0, 4, 4), &bytes);
    assert_eq!(doc.pixel_format(), PixelFormat::RGBA8);
    doc.layers.push(Layer::new("pattern", LayerContent::Raster(surface)));
    doc
}

/// An app with the 4×4 pattern open.
fn app_with_pattern() -> PhotocraftApp {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services());
    app.session.open_document(pattern_document(), None);
    app.sync_views();
    app
}

fn empty_app() -> PhotocraftApp {
    PhotocraftApp::new(photocraft_engine::Session::new(), services())
}

fn call(app: &mut PhotocraftApp, method: &str, params: Value) -> Value {
    let ctx = egui::Context::default();
    let (req, _rx) = ControlRequest::new(method, params);
    match handle(app, &ctx, &req) {
        Outcome::Done(reply) => reply,
        _ => panic!("{method} did not answer at once"),
    }
}

fn ok(app: &mut PhotocraftApp, method: &str, params: Value) -> Value {
    let reply = call(app, method, params.clone());
    assert_eq!(reply["ok"], true, "{method} {params}: {reply}");
    reply["result"].clone()
}

fn error(app: &mut PhotocraftApp, method: &str, params: Value) -> String {
    let reply = call(app, method, params.clone());
    assert_eq!(reply["ok"], false, "{method} {params} should fail: {reply}");
    reply["error"].as_str().unwrap_or_else(|| panic!("{method}: no error text in {reply}")).to_owned()
}

fn execute(app: &mut PhotocraftApp, command: &str, params: Value) -> Value {
    ok(app, "engine.execute", json!({"command": command, "params": params}))
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn unb64(text: &Value) -> Vec<u8> {
    let text = text.as_str().expect("base64 text");
    assert!(!text.starts_with("data:"), "no data: prefix");
    base64::engine::general_purpose::STANDARD.decode(text).expect("valid base64")
}

fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let image = Image::from_u8(width, height, ChannelLayout::Rgba, rgba.to_vec()).unwrap();
    photocraft_codecs::encode(&image, Format::Png, &Default::default()).unwrap()
}

/// A PNG in one colour.
fn flat_png(width: u32, height: u32, colour: [u8; 4]) -> Vec<u8> {
    png(width, height, &colour.repeat((width * height) as usize))
}

/// (width, height, layout, bytes) of a decoded PNG.
fn decode(png: &[u8]) -> (u32, u32, ChannelLayout, Vec<u8>) {
    let image = photocraft_codecs::decode(png).expect("a PNG");
    (image.width(), image.height(), image.layout(), image.into_data())
}

fn rgba_of(png: &[u8]) -> (u32, u32, Vec<u8>) {
    let image = photocraft_codecs::decode(png).expect("a PNG").convert(ChannelLayout::Rgba, CodecSample::U8);
    (image.width(), image.height(), image.into_data())
}

fn px(data: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [data[i], data[i + 1], data[i + 2], data[i + 3]]
}

/// The composite at a document pixel, as 8-bit values.
fn composite(app: &mut PhotocraftApp, x: u32, y: u32) -> [u8; 4] {
    let v: Vec<f32> = serde_json::from_value(execute(app, "document.pixel", json!({"x": x, "y": y}))).unwrap();
    let c = |i: usize| (v[i] * 255.0).round() as u8;
    [c(0), c(1), c(2), if v.len() > 3 { c(3) } else { 255 }]
}

fn close(a: [u8; 4], b: [u8; 4]) -> bool {
    a.iter().zip(&b).all(|(p, q)| p.abs_diff(*q) <= 1)
}

fn layer_count(app: &PhotocraftApp) -> usize {
    app.session.active().unwrap().doc.layer_count()
}

fn steps(app: &PhotocraftApp) -> usize {
    app.session.active().unwrap().history.past_len()
}

fn select(app: &mut PhotocraftApp, x: u32, y: u32, width: u32, height: u32) {
    execute(app, "select.rect", json!({"x": x, "y": y, "width": width, "height": height}));
}

// ---------- c0.capture ----------

#[test]
fn capture_of_the_document_decodes_to_the_same_pixels() {
    let mut app = app_with_pattern();
    let result = ok(&mut app, "c0.capture", json!({"scope": "document"}));
    assert_eq!(result["rect"], json!({"x": 0, "y": 0, "width": 4, "height": 4}));
    assert_eq!(result["scale"], 1.0);
    let (w, h, data) = rgba_of(&unb64(&result["png"]));
    assert_eq!((w, h), (4, 4));
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(px(&data, w, x, y), pattern(x, y), "pixel {x},{y}");
        }
    }
}

#[test]
fn capture_shows_the_visible_layers_only() {
    let mut app = app_with_pattern();
    ok(&mut app, "c0.place", json!({"png": b64(&flat_png(1, 1, [250, 0, 0, 255])), "name": "red", "rect": {"x": 0, "y": 0, "width": 4, "height": 4}}));
    let (w, _, data) = rgba_of(&unb64(&ok(&mut app, "c0.capture", json!({"scope": "document"}))["png"]));
    assert_eq!(px(&data, w, 2, 2), [250, 0, 0, 255], "the red layer covers the pattern");
    let layer = app.session.active().unwrap().active_layer.unwrap();
    app.session.edit("hide", |doc, _| {
        doc.layer_mut(layer).unwrap().visible = false;
        Ok(())
    })
    .unwrap();
    let (w, _, data) = rgba_of(&unb64(&ok(&mut app, "c0.capture", json!({"scope": "document"}))["png"]));
    assert_eq!(px(&data, w, 2, 2), pattern(2, 2), "hidden, it is not in the capture");
}

#[test]
fn capture_of_the_selection_is_cropped_to_its_bounds() {
    let mut app = app_with_pattern();
    select(&mut app, 1, 1, 2, 3);
    let result = ok(&mut app, "c0.capture", json!({"scope": "selection"}));
    assert_eq!(result["rect"], json!({"x": 1, "y": 1, "width": 2, "height": 3}));
    assert_eq!(result["scale"], 1.0);
    let (w, h, data) = rgba_of(&unb64(&result["png"]));
    assert_eq!((w, h), (2, 3));
    for y in 0..3 {
        for x in 0..2 {
            assert_eq!(px(&data, w, x, y), pattern(x + 1, y + 1), "pixel {x},{y} of the crop");
        }
    }
}

#[test]
fn max_side_downsizes_and_reports_the_scale() {
    let mut app = app_with_pattern();
    let small = ok(&mut app, "c0.capture", json!({"scope": "document", "maxSide": 2}));
    let (w, h, _) = rgba_of(&unb64(&small["png"]));
    assert_eq!((w, h), (2, 2));
    assert_eq!(small["scale"], 0.5);
    assert_eq!(small["rect"], json!({"x": 0, "y": 0, "width": 4, "height": 4}), "the rect stays in document pixels");
    for max_side in [json!(0), json!(4), json!(100), Value::Null] {
        let result = ok(&mut app, "c0.capture", json!({"scope": "document", "maxSide": max_side}));
        assert_eq!(rgba_of(&unb64(&result["png"])).0, 4, "maxSide {max_side}: full size");
        assert_eq!(result["scale"], 1.0, "maxSide {max_side}");
    }
    select(&mut app, 0, 0, 4, 2);
    let half = ok(&mut app, "c0.capture", json!({"scope": "selection", "maxSide": 2}));
    let (w, h, _) = rgba_of(&unb64(&half["png"]));
    assert_eq!((w, h), (2, 1), "the crop is what is scaled");
    assert_eq!(half["scale"], 0.5);
    assert_eq!(half["rect"], json!({"x": 0, "y": 0, "width": 4, "height": 2}));
}

// ---------- c0.selectionMask ----------

#[test]
fn the_mask_is_one_channel_the_size_of_the_rect_white_inside_and_black_outside() {
    let mut app = app_with_pattern();
    select(&mut app, 1, 1, 2, 2);
    let result = ok(&mut app, "c0.selectionMask", json!({"rect": {"x": 0, "y": 0, "width": 4, "height": 4}}));
    let (w, h, layout, data) = decode(&unb64(&result["png"]));
    assert_eq!((w, h), (4, 4));
    assert_eq!(layout, ChannelLayout::Gray, "a single channel");
    assert_eq!(data.len(), 16);
    for y in 0..4u32 {
        for x in 0..4u32 {
            let inside = (1..3).contains(&x) && (1..3).contains(&y);
            assert_eq!(data[(y * 4 + x) as usize], if inside { 255 } else { 0 }, "pixel {x},{y}");
        }
    }
}

#[test]
fn the_mask_follows_a_rect_that_leaves_the_canvas_and_keeps_partial_selection_as_grey() {
    let mut app = app_with_pattern();
    select(&mut app, 0, 0, 4, 4);
    app.session
        .edit("half", |doc, _| {
            let mut selection = doc.selection.take().expect("selected");
            selection.write_region(Rect::new(1, 1, 2, 2), &[0.5]);
            doc.selection = Some(selection);
            Ok(())
        })
        .unwrap();
    let result = ok(&mut app, "c0.selectionMask", json!({"rect": {"x": -1, "y": -1, "width": 6, "height": 6}}));
    let (w, h, layout, data) = decode(&unb64(&result["png"]));
    assert_eq!((w, h, layout), (6, 6, ChannelLayout::Gray));
    assert_eq!(data[0], 0, "outside the canvas is not selected");
    assert_eq!(data[(3 * 6 + 3) as usize], 255, "document pixel 2,2 is selected");
    let grey = data[(2 * 6 + 2) as usize];
    assert!((127..=128).contains(&grey), "document pixel 1,1 is half selected: {grey}");
}

// ---------- c0.place ----------

#[test]
fn place_with_a_rect_adds_one_layer_with_that_name_at_that_rect_in_one_step() {
    let mut app = app_with_pattern();
    let (layers, before) = (layer_count(&app), steps(&app));
    let below = app.session.active().unwrap().active_layer.unwrap();
    let image = png(2, 2, &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]);
    let result = ok(&mut app, "c0.place", json!({"png": b64(&image), "name": "Sunset", "rect": {"x": 1, "y": 1, "width": 2, "height": 2}}));
    let id = result["layer"].as_u64().expect("a layer id");
    assert_eq!(layer_count(&app), layers + 1);
    assert_eq!(steps(&app), before + 1, "one history step");
    let doc = &app.session.active().unwrap().doc;
    let placed = doc.layer(photocraft_doc::LayerId(id)).expect("the layer exists");
    assert_eq!(placed.name, "Sunset");
    assert!(matches!(placed.content, LayerContent::Raster(_)), "a pixel layer");
    assert!(placed.mask.is_none());
    let order: Vec<_> = doc.layers.iter().map(|l| l.id).collect();
    let at = order.iter().position(|l| l.0 == id).unwrap();
    assert_eq!(order[at - 1], below, "above the layer that was active");
    assert_eq!(app.session.active().unwrap().active_layer.map(|l| l.0), Some(id), "and it is the active one");
    let bounds = placed.surface().unwrap().content_bounds();
    assert_eq!(bounds, Rect::new(1, 1, 3, 3), "at the rect");
    assert!(close(composite(&mut app, 1, 1), [255, 0, 0, 255]));
    assert!(close(composite(&mut app, 2, 1), [0, 255, 0, 255]));
    assert!(close(composite(&mut app, 1, 2), [0, 0, 255, 255]));
    assert!(close(composite(&mut app, 2, 2), [255, 255, 255, 255]));
    assert!(close(composite(&mut app, 0, 0), pattern(0, 0)), "outside the rect the pattern shows");
    assert!(close(composite(&mut app, 3, 3), pattern(3, 3)));

    execute(&mut app, "edit.undo", json!({}));
    assert_eq!(layer_count(&app), layers, "undo removes it");
    assert_eq!(steps(&app), before);
    assert!(app.session.active().unwrap().doc.layer(photocraft_doc::LayerId(id)).is_none());
    assert!(close(composite(&mut app, 1, 1), pattern(1, 1)));
}

#[test]
fn the_history_step_is_named_after_the_image() {
    let mut app = app_with_pattern();
    ok(&mut app, "c0.place", json!({"png": b64(&flat_png(1, 1, [9, 9, 9, 255])), "name": "A moody sky"}));
    assert_eq!(app.session.active().unwrap().history.undo_label(), Some("A moody sky"));
}

#[test]
fn place_scales_the_image_to_cover_the_rect() {
    let mut app = app_with_pattern();
    ok(&mut app, "c0.place", json!({"png": b64(&flat_png(1, 1, [200, 100, 50, 255])), "name": "flat", "rect": {"x": 0, "y": 0, "width": 4, "height": 4}}));
    for (x, y) in [(0, 0), (3, 0), (0, 3), (3, 3), (2, 1)] {
        assert!(close(composite(&mut app, x, y), [200, 100, 50, 255]), "pixel {x},{y}");
    }
    let bounds = app.session.active().unwrap().doc.layers.last().unwrap().surface().unwrap().content_bounds();
    assert_eq!(bounds, Rect::new(0, 0, 4, 4));
}

#[test]
fn place_without_a_rect_fits_the_whole_canvas() {
    let mut app = app_with_pattern();
    // 2×1: scaled to the canvas width, centred in height, in its own proportions.
    ok(&mut app, "c0.place", json!({"png": b64(&flat_png(2, 1, [10, 200, 10, 255])), "name": "wide", "rect": null}));
    let bounds = app.session.active().unwrap().doc.layers.last().unwrap().surface().unwrap().content_bounds();
    assert_eq!(bounds, Rect::new(0, 1, 4, 3));
    ok(&mut app, "c0.place", json!({"png": b64(&flat_png(8, 8, [200, 10, 10, 255])), "name": "square"}));
    let bounds = app.session.active().unwrap().doc.layer(app.session.active().unwrap().active_layer.unwrap()).unwrap().surface().unwrap().content_bounds();
    assert_eq!(bounds, Rect::new(0, 0, 4, 4));
}

#[test]
fn clip_to_selection_gives_the_layer_a_mask_from_the_selection() {
    let mut app = app_with_pattern();
    select(&mut app, 0, 0, 2, 2);
    let (layers, before) = (layer_count(&app), steps(&app));
    let result = ok(
        &mut app,
        "c0.place",
        json!({"png": b64(&flat_png(1, 1, [255, 0, 0, 255])), "name": "clipped", "rect": {"x": 0, "y": 0, "width": 4, "height": 4}, "clipToSelection": true}),
    );
    assert_eq!(layer_count(&app), layers + 1);
    assert_eq!(steps(&app), before + 1, "the layer and its mask are one step");
    let id = photocraft_doc::LayerId(result["layer"].as_u64().unwrap());
    let layer = app.session.active().unwrap().doc.layer(id).unwrap();
    assert!(layer.mask.is_some(), "the layer has a mask");
    assert!(close(composite(&mut app, 1, 1), [255, 0, 0, 255]), "the image shows inside the selection");
    assert!(close(composite(&mut app, 3, 3), pattern(3, 3)), "and not outside it");
    assert!(close(composite(&mut app, 2, 0), pattern(2, 0)));
    execute(&mut app, "edit.undo", json!({}));
    assert_eq!(layer_count(&app), layers);
}

#[test]
fn as_document_opens_a_new_document_and_leaves_the_active_one_alone() {
    let mut app = app_with_pattern();
    let layers = layer_count(&app);
    let result = ok(&mut app, "c0.place", json!({"png": b64(&flat_png(3, 2, [1, 2, 3, 255])), "name": "Generated 1", "asDocument": true}));
    assert_eq!(result["document"], 1);
    assert!(result.get("layer").is_none());
    assert_eq!(app.session.documents().len(), 2);
    let doc = &app.session.documents()[1].doc;
    assert_eq!((doc.size.width, doc.size.height), (3, 2));
    assert_eq!(doc.name, "Generated 1");
    assert_eq!(app.session.documents()[0].doc.layer_count(), layers, "the first document is untouched");
    assert!(close(composite(&mut app, 1, 1), [1, 2, 3, 255]), "the new document is the active one");
}

#[test]
fn as_document_needs_no_open_document() {
    let mut app = empty_app();
    let result = ok(&mut app, "c0.place", json!({"png": b64(&flat_png(2, 2, [5, 5, 5, 255])), "name": "First", "asDocument": true}));
    assert_eq!(result["document"], 0);
    assert_eq!(app.session.documents().len(), 1);
}

// ---------- c0.openFile and the file dialog ----------

#[test]
fn open_file_opens_png_bytes_as_a_document_of_the_right_size() {
    let mut app = app_with_pattern();
    let result = ok(&mut app, "c0.openFile", json!({"name": "photo.png", "bytes": b64(&flat_png(5, 3, [9, 8, 7, 255]))}));
    assert_eq!(result["document"], 1);
    assert_eq!(app.session.documents().len(), 2);
    let doc = &app.session.documents()[1].doc;
    assert_eq!((doc.size.width, doc.size.height), (5, 3));
    assert_eq!(doc.name, "photo.png");
    assert_eq!(app.session.active().unwrap().doc.size.width, 5, "in its own tab, now the active one");
}

#[test]
fn open_file_takes_the_formats_the_picker_does() {
    let source = pattern_document();
    for extension in ["psd", "tiff", "png", "jpg", "bmp", "webp"] {
        let exported = photocraft_io::export(&source, extension, &Default::default()).unwrap_or_else(|e| panic!("exporting {extension}: {e}"));
        let mut app = empty_app();
        let result = ok(&mut app, "c0.openFile", json!({"name": format!("file.{extension}"), "bytes": b64(&exported.bytes)}));
        assert_eq!(result["document"], 0, "{extension}");
        let doc = &app.session.documents()[0].doc;
        assert_eq!((doc.size.width, doc.size.height), (4, 4), "{extension}");
    }
}

#[test]
fn open_file_with_garbage_is_an_error_and_opens_nothing() {
    let mut app = app_with_pattern();
    for bytes in [b"this is not an image".to_vec(), vec![0u8; 64], b"\x89PNG\r\n\x1a\nbroken".to_vec()] {
        let message = error(&mut app, "c0.openFile", json!({"name": "bad.png", "bytes": b64(&bytes)}));
        assert!(message.contains("bad.png"), "{message}");
        assert_eq!(app.session.documents().len(), 1);
    }
    error(&mut app, "c0.openFile", json!({"name": "bad.png", "bytes": "%%% not base64"}));
    error(&mut app, "c0.openFile", json!({"name": "bad.png", "bytes": ""}));
    error(&mut app, "c0.openFile", json!({"name": "", "bytes": b64(b"x")}));
    assert_eq!(app.session.documents().len(), 1);
}

/// A dialog service that shows nothing and never answers: the card that stayed open.
fn unanswered_dialog(asked: &Rc<RefCell<Vec<FileDialogRequest>>>) -> Services {
    let log = asked.clone();
    let held = Rc::new(RefCell::new(Vec::new()));
    Services {
        file_dialog: Some(Box::new(move |request, _parent, reply| {
            log.borrow_mut().push(request);
            held.borrow_mut().push(reply);
        })),
        ..services()
    }
}

#[test]
fn open_file_never_asks_for_a_file_dialog() {
    let asked = Rc::new(RefCell::new(Vec::new()));
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), unanswered_dialog(&asked));
    ok(&mut app, "c0.openFile", json!({"name": "a.png", "bytes": b64(&flat_png(2, 2, [1, 1, 1, 255]))}));
    assert!(!app.file_dialog_open());
    assert!(asked.borrow().is_empty(), "the picker was not asked: {:?}", asked.borrow());
    ok(&mut app, "c0.place", json!({"png": b64(&flat_png(2, 2, [1, 1, 1, 255])), "name": "b", "asDocument": true}));
    assert!(asked.borrow().is_empty() && !app.file_dialog_open());
}

#[test]
fn cancel_file_dialog_dismisses_a_pending_dialog_so_the_next_one_can_open() {
    let asked = Rc::new(RefCell::new(Vec::new()));
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), unanswered_dialog(&asked));
    assert_eq!(ok(&mut app, "c0.cancelFileDialog", json!({}))["cancelled"], false, "nothing to dismiss");
    app.open_dialog_file().expect("opens a dialog");
    assert!(app.file_dialog_open());
    assert_eq!(app.open_dialog_file().unwrap_err(), "a file dialog is already open", "the symptom the host saw");
    assert_eq!(ok(&mut app, "c0.cancelFileDialog", json!({}))["cancelled"], true);
    assert!(!app.file_dialog_open());
    assert_eq!(ok(&mut app, "c0.cancelFileDialog", json!(null))["cancelled"], false, "a second cancel finds nothing");
    app.open_dialog_file().expect("a new dialog can open after the cancel");
}

// ---------- errors ----------

#[test]
fn without_a_document_each_command_names_what_is_missing() {
    let mut app = empty_app();
    let image = b64(&flat_png(1, 1, [1, 1, 1, 255]));
    for (method, params) in [
        ("c0.capture", json!({"scope": "document"})),
        ("c0.capture", json!({"scope": "selection"})),
        ("c0.selectionMask", json!({"rect": {"x": 0, "y": 0, "width": 2, "height": 2}})),
        ("c0.place", json!({"png": image, "name": "x"})),
        ("c0.place", json!({"png": image, "name": "x", "clipToSelection": true})),
    ] {
        assert_eq!(error(&mut app, method, params.clone()), "no document open", "{method} {params}");
    }
    assert_eq!(app.session.documents().len(), 0);
}

#[test]
fn without_a_selection_each_command_says_no_selection_and_changes_nothing() {
    let mut app = app_with_pattern();
    let (layers, before) = (layer_count(&app), steps(&app));
    let image = b64(&flat_png(1, 1, [1, 1, 1, 255]));
    for (method, params) in [
        ("c0.capture", json!({"scope": "selection"})),
        ("c0.selectionMask", json!({"rect": {"x": 0, "y": 0, "width": 2, "height": 2}})),
        ("c0.place", json!({"png": image, "name": "x", "clipToSelection": true})),
    ] {
        assert_eq!(error(&mut app, method, params.clone()), "no selection", "{method} {params}");
    }
    assert_eq!((layer_count(&app), steps(&app)), (layers, before));
}

#[test]
fn bad_parameters_are_refused_before_anything_changes() {
    let mut app = app_with_pattern();
    let (layers, before) = (layer_count(&app), steps(&app));
    let image = b64(&flat_png(1, 1, [1, 1, 1, 255]));
    let cases = [
        ("c0.capture", json!({})),
        ("c0.capture", json!({"scope": "layer"})),
        ("c0.capture", json!({"scope": "document", "maxSide": "big"})),
        ("c0.selectionMask", json!({})),
        ("c0.selectionMask", json!({"rect": {"x": 0, "y": 0, "width": 0, "height": 2}})),
        ("c0.selectionMask", json!({"rect": {"x": 0, "y": 0, "width": 99999, "height": 99999}})),
        ("c0.place", json!({"png": image})),
        ("c0.place", json!({"png": image, "name": "  "})),
        ("c0.place", json!({"png": "!!", "name": "x"})),
        ("c0.place", json!({"png": b64(b"not a png"), "name": "x"})),
        ("c0.place", json!({"png": image, "name": "x", "rect": {"x": 0, "y": 0, "width": -1, "height": 2}})),
        ("c0.place", json!({"png": image, "name": "x", "clipToSelection": "yes"})),
        ("c0.nothing", json!({})),
    ];
    for (method, params) in cases {
        let reply = call(&mut app, method, params.clone());
        assert_eq!(reply["ok"], false, "{method} {params}: {reply}");
    }
    assert_eq!((layer_count(&app), steps(&app), app.session.documents().len()), (layers, before, 1));
}
