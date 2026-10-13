//! The `c0.*` requests of the C0VIBE host: what an image-generation panel needs from the editor.
//!
//! This file is not upstream PhotoCraft's: it belongs to the C0VIBE host shell (see
//! `c0/photocraft-host/README.md`). `control::handle` hands every method that starts with `c0.` to
//! [`handle`], so the desktop socket and the page bridge answer them alike and a native test can run
//! them. Nothing here reads or writes a path or a file: images come and go as bytes.
//!
//! | Method | Params | Result |
//! |---|---|---|
//! | `c0.capture` | `{scope: "document" \| "selection", maxSide?}` | `{png, rect, scale}` |
//! | `c0.selectionMask` | `{rect}` | `{png}` (8-bit grey) |
//! | `c0.place` | `{png, name, rect?, clipToSelection?, asDocument?}` | `{layer}` or `{document}` |
//! | `c0.openFile` | `{name, bytes}` | `{document}` |
//! | `c0.cancelFileDialog` | `{}` | `{cancelled}` |
//!
//! `png` and `bytes` are base64 (standard alphabet, no `data:` prefix). Rectangles are in document
//! pixels.

use std::sync::Arc;

use base64::Engine as _;
use photocraft_algo::resample::{Resample, crop_surface, resize_surface_in_canvas, translate_surface};
use photocraft_codecs::{ChannelLayout, Format, Image, SampleType};
use photocraft_color::PixelFormat;
use photocraft_doc::{Layer, LayerContent, LayerMask};
use photocraft_geom::Rect;
use photocraft_raster::Surface;
use serde_json::{Value, json};

use crate::PhotocraftApp;
use crate::file_open::display_name;

/// The error of a request that needs a document and has none.
const NO_DOCUMENT: &str = "no document open";
/// The error of a request that needs a selection and has none.
const NO_SELECTION: &str = "no selection";
/// The longest side of a rectangle or an image a request may ask for (a request must not be able to
/// ask for an allocation the engine would refuse later).
const MAX_SIDE: i64 = 32_768;
/// The most pixels a rectangle or an image of a request may hold.
const MAX_PIXELS: u64 = 120_000_000;

/// Answers a `c0.*` request.
pub fn handle(app: &mut PhotocraftApp, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "c0.capture" => capture(app, params),
        "c0.selectionMask" => selection_mask(app, params),
        "c0.place" => place(app, params),
        "c0.openFile" => open_file(app, params),
        "c0.cancelFileDialog" => Ok(json!({"cancelled": cancel_file_dialog(app)})),
        other => Err(format!("unknown method `{other}`")),
    }
}

/// The methods of this module, for the host contract test and the bridge's rules.
pub const METHODS: [&str; 5] = ["c0.capture", "c0.selectionMask", "c0.place", "c0.openFile", "c0.cancelFileDialog"];

/// Dismisses the in-page file dialog, if one is open. The caller that asked for it is answered as
/// for a cancel ("cancelled", which is not shown). Returns whether there was one.
pub fn cancel_file_dialog(app: &mut PhotocraftApp) -> bool {
    app.file_dialog.take().is_some()
}

// ---------- parameters ----------

fn text(p: &Value, key: &str) -> Result<String, String> {
    match p.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_owned()),
        _ => Err(format!("`{key}` must be a non-empty string")),
    }
}

fn flag(p: &Value, key: &str) -> Result<bool, String> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("`{key}` must be a boolean")),
    }
}

/// A whole number from JSON (`512` or `512.0`).
fn whole(v: &Value, what: &str) -> Result<i64, String> {
    v.as_i64()
        .or_else(|| v.as_f64().filter(|f| f.is_finite() && f.fract() == 0.0 && f.abs() < 1e12).map(|f| f as i64))
        .ok_or_else(|| format!("{what} must be a whole number"))
}

/// A rectangle `{x, y, width, height}` in document pixels.
fn rect_param(v: &Value, what: &str) -> Result<Rect, String> {
    let field = |key: &str| whole(v.get(key).unwrap_or(&Value::Null), &format!("{what}.{key}"));
    let (x, y, w, h) = (field("x")?, field("y")?, field("width")?, field("height")?);
    if w < 1 || h < 1 {
        return Err(format!("{what}.width and {what}.height must be at least 1"));
    }
    if w > MAX_SIDE || h > MAX_SIDE || (w as u64) * (h as u64) > MAX_PIXELS || x.abs() > MAX_SIDE * 4 || y.abs() > MAX_SIDE * 4 {
        return Err(format!("{what} is larger than {MAX_SIDE} pixels a side or {} million pixels", MAX_PIXELS / 1_000_000));
    }
    Ok(Rect::new(x as i32, y as i32, (x + w) as i32, (y + h) as i32))
}

fn bytes_param(p: &Value, key: &str) -> Result<Vec<u8>, String> {
    let Some(Value::String(s)) = p.get(key) else { return Err(format!("`{key}` must be base64 text")) };
    // A `data:` URL is accepted on the way in, although none is ever sent out.
    let s = s.split_once(";base64,").map_or(s.as_str(), |(_, rest)| rest);
    let bytes = base64::engine::general_purpose::STANDARD.decode(s.trim()).map_err(|e| format!("`{key}` is not valid base64: {e}"))?;
    if bytes.is_empty() {
        return Err(format!("`{key}` is empty"));
    }
    Ok(bytes)
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

// ---------- images ----------

fn encode_png(width: u32, height: u32, layout: ChannelLayout, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let image = Image::from_u8(width, height, layout, data).map_err(|e| e.to_string())?;
    photocraft_codecs::encode(&image, Format::Png, &Default::default()).map_err(|e| e.to_string())
}

/// A decoded image as straight RGBA8: (width, height, pixels).
fn decode_rgba8(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let image = photocraft_codecs::decode(bytes).map_err(|e| format!("the image could not be read: {e}"))?;
    let (w, h) = (image.width(), image.height());
    if w == 0 || h == 0 || i64::from(w) > MAX_SIDE || i64::from(h) > MAX_SIDE || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(format!("the image is {w}×{h}: larger than {MAX_SIDE} pixels a side or {} million pixels", MAX_PIXELS / 1_000_000));
    }
    let rgba = image.convert(ChannelLayout::Rgba, SampleType::U8);
    Ok((w, h, rgba.into_data()))
}

/// Where a selection is: its bounds on the canvas, `Err` when nothing is selected.
fn selection_bounds(doc: &photocraft_doc::Document) -> Result<Rect, String> {
    let selection = doc.selection.as_ref().ok_or(NO_SELECTION)?;
    let bounds = selection.content_bounds().intersect(&doc.bounds());
    if bounds.is_empty() { Err(NO_SELECTION.into()) } else { Ok(bounds) }
}

// ---------- c0.capture ----------

fn capture(app: &PhotocraftApp, p: &Value) -> Result<Value, String> {
    let scope = match p.get("scope").and_then(Value::as_str) {
        Some(s @ ("document" | "selection")) => s,
        _ => return Err("`scope` must be \"document\" or \"selection\"".into()),
    };
    let max_side = match p.get("maxSide") {
        None | Some(Value::Null) => 0,
        Some(v) => u32::try_from(whole(v, "maxSide")?.max(0)).map_err(|_| "maxSide is too large".to_owned())?,
    };
    let doc = &app.session.active().ok_or(NO_DOCUMENT)?.doc;
    let rect = if scope == "document" { doc.bounds() } else { selection_bounds(doc)? };
    let (rw, rh) = (rect.width(), rect.height());
    let longest = rw.max(rh);
    // The composite the user sees, not the interface: the engine's own thumbnail renderer (the one
    // behind `doc.render`), over the area asked for, averaged down when `maxSide` is smaller.
    let side = if max_side == 0 || max_side >= longest { longest } else { max_side };
    let (w, h) = if side == longest {
        (rw, rh)
    } else {
        let k = f64::from(side) / f64::from(longest);
        (((f64::from(rw) * k).round() as u32).max(1), ((f64::from(rh) * k).round() as u32).max(1))
    };
    let image = photocraft_compose::render_reduced_rect(doc, rect, w, h).to_rgba8();
    let (iw, ih) = (image.width, image.height);
    let png = encode_png(iw, ih, ChannelLayout::Rgba, image.pixels)?;
    let scale = if side == longest { 1.0 } else { f64::from(iw.max(ih)) / f64::from(longest) };
    Ok(json!({
        "png": b64(&png),
        "rect": {"x": rect.x0, "y": rect.y0, "width": rw, "height": rh},
        "scale": scale,
    }))
}

// ---------- c0.selectionMask ----------

fn selection_mask(app: &PhotocraftApp, p: &Value) -> Result<Value, String> {
    let rect = rect_param(p.get("rect").unwrap_or(&Value::Null), "rect")?;
    let doc = &app.session.active().ok_or(NO_DOCUMENT)?.doc;
    // Checked after the document so the error names what is missing first.
    selection_bounds(doc)?;
    let selection = doc.selection.as_ref().ok_or(NO_SELECTION)?;
    let channels = selection.channels().max(1);
    let values = selection.read_region(rect);
    let grey: Vec<u8> = values.chunks_exact(channels).map(|px| (px[0].clamp(0.0, 1.0) * 255.0).round() as u8).collect();
    let png = encode_png(rect.width(), rect.height(), ChannelLayout::Gray, grey)?;
    Ok(json!({"png": b64(&png)}))
}

// ---------- c0.place ----------

/// The rectangle an image of `iw`×`ih` takes when none is given: as large as fits on the canvas,
/// centred, in the image's own proportions.
fn fitted(canvas: Rect, iw: u32, ih: u32) -> Rect {
    let (cw, ch) = (f64::from(canvas.width()), f64::from(canvas.height()));
    let k = (cw / f64::from(iw)).min(ch / f64::from(ih));
    let (w, h) = (((f64::from(iw) * k).round() as i32).max(1), ((f64::from(ih) * k).round() as i32).max(1));
    let x = canvas.x0 + (canvas.width() as i32 - w).div_euclid(2);
    let y = canvas.y0 + (canvas.height() as i32 - h).div_euclid(2);
    Rect::new(x, y, x + w, y + h)
}

/// The image's pixels as a surface in `format` covering exactly `target`.
fn image_surface(width: u32, height: u32, rgba: &[u8], target: Rect, format: PixelFormat) -> Surface {
    let source = Rect::new(0, 0, width as i32, height as i32);
    let mut surface = Surface::from_interleaved(PixelFormat::RGBA8, source, rgba);
    let (tw, th) = (target.width() as u32, target.height() as u32);
    if (tw, th) != (width, height) {
        let (sx, sy) = (f64::from(tw) / f64::from(width), f64::from(th) / f64::from(height));
        // Photoshop's choice: Bicubic Smoother to enlarge, Bicubic Sharper to reduce.
        let filter = if sx * sy > 1.0 { Resample::BicubicSmoother } else { Resample::BicubicSharper };
        // The edges repeat outwards, so the image stays opaque to its border instead of fading
        // into the transparent surround.
        surface = crop_surface(&resize_surface_in_canvas(&surface, sx, sy, filter, source), Rect::new(0, 0, tw as i32, th as i32));
    }
    let surface = translate_surface(&surface, target.x0, target.y0);
    if format == PixelFormat::RGBA8 {
        return surface;
    }
    // Another colour mode or depth: through straight RGBA, as the compositor converts.
    let values = surface.read_region(target);
    let n = format.channels();
    let mut data = vec![0.0f32; target.width() as usize * target.height() as usize * n];
    for (rgba, out) in values.chunks_exact(4).zip(data.chunks_exact_mut(n)) {
        photocraft_raster::from_rgba_into(&format, [rgba[0], rgba[1], rgba[2], rgba[3]], out);
    }
    let mut converted = Surface::new(format);
    converted.write_region(target, &data);
    converted
}

fn place(app: &mut PhotocraftApp, p: &Value) -> Result<Value, String> {
    let name = text(p, "name")?;
    let as_document = flag(p, "asDocument")?;
    let clip = flag(p, "clipToSelection")?;
    let rect = match p.get("rect") {
        None | Some(Value::Null) => None,
        Some(v) => Some(rect_param(v, "rect")?),
    };
    let png = bytes_param(p, "png")?;
    if as_document {
        return open_as_document(app, &name, &png);
    }
    let st = app.session.active().ok_or(NO_DOCUMENT)?;
    let selection = if clip {
        selection_bounds(&st.doc)?;
        st.doc.selection.clone()
    } else {
        None
    };
    let (width, height, rgba) = decode_rgba8(&png)?;
    let target = rect.unwrap_or_else(|| fitted(st.doc.bounds(), width, height));
    let surface = image_surface(width, height, &rgba, target, st.doc.pixel_format());
    let mut layer = Layer::new(name.clone(), LayerContent::Raster(surface));
    if let Some(surface) = selection {
        layer.mask = Some(LayerMask { surface, ..LayerMask::reveal_all() });
    }
    // One step, named after the image: the layer and its mask come and go together.
    let id = app
        .session
        .edit(&name, |doc, active| {
            let id = doc.insert_above(*active, layer);
            *active = Some(id);
            Ok(id)
        })
        .map_err(|e| e.to_string())?;
    app.sync_views();
    Ok(json!({"layer": id.0}))
}

/// Opens an image's bytes as a new document called `name`.
fn open_as_document(app: &mut PhotocraftApp, name: &str, png: &[u8]) -> Result<Value, String> {
    // The importer goes by content; the extension only has to be one it knows.
    let index = open_bytes(app, &format!("{name}.png"), png)?;
    // The new document is the active one.
    if let Some(st) = app.session.active_mut() {
        Arc::make_mut(&mut st.doc).name = name.to_owned();
    }
    app.sync_views();
    Ok(json!({"document": index}))
}

// ---------- c0.openFile ----------

/// Opens `bytes` as File › Open does once the picker has handed them over, and returns the new
/// document's index. A file that is not a document (a brush set goes to its library) is an error here.
fn open_bytes(app: &mut PhotocraftApp, name: &str, bytes: &[u8]) -> Result<usize, String> {
    let before = app.session.documents().len();
    app.open_bytes(&display_name(name), bytes).map_err(|e| format!("{}: {e}", display_name(name)))?;
    let after = app.session.documents().len();
    if after <= before {
        return Err(format!("{}: did not open as a document", display_name(name)));
    }
    Ok(after - 1)
}

fn open_file(app: &mut PhotocraftApp, p: &Value) -> Result<Value, String> {
    let name = text(p, "name")?;
    let bytes = bytes_param(p, "bytes")?;
    let index = open_bytes(app, &name, &bytes)?;
    Ok(json!({"document": index}))
}
