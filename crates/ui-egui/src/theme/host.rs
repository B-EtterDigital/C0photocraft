//! A palette set at run time by the application that embeds PhotoCraft.
//!
//! This file is not upstream PhotoCraft's: it belongs to the C0VIBE host shell (see
//! `c0/photocraft-host/README.md`). An embedding host picks the nearest preset and then lays its own
//! colours and radii over it, so the guest shows the host's light and dark palette instead of a
//! preset's.
//!
//! The overrides are kept in egui's context data and [`super::apply`] lays them over every preset it
//! applies, so they stay in force when the preset changes, when a document opens and on every frame,
//! and egui's own widget visuals (which `apply` derives from the tokens) follow them too. Keys are the
//! ones [`super::live::apply_overrides`] accepts: `Tokens` field names, with `"#rrggbb"` or
//! `"#rrggbbaa"` for a colour and a number for a radius.

use super::{ThemeKind, Tokens};
use serde_json::{Map, Value, json};
use std::sync::Arc;

/// The tokens that take a number (corner radii, in points) instead of a colour.
const RADII: [&str; 3] = ["radius_sm", "radius", "radius_lg"];

const SHAPE: &str = "themeTokens must be an object of token names to \"#rrggbb\", \"#rrggbbaa\" or a radius number, or null to clear";

fn store() -> egui::Id {
    egui::Id::new("photocraft-theme-host")
}

/// What became of each key of an override set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Keys now in force.
    pub applied: Vec<String>,
    /// Keys that are not tokens (a token renamed or removed upstream shows up here).
    pub unknown: Vec<String>,
    /// Tokens whose value is not a colour or a radius; they keep the preset's value.
    pub invalid: Vec<String>,
}

impl Report {
    pub fn to_json(&self) -> Value {
        json!({"applied": self.applied, "unknown": self.unknown, "invalid": self.invalid})
    }
}

/// The override set in force (only the keys that apply), if any.
pub fn current(ctx: &egui::Context) -> Option<Arc<Value>> {
    ctx.data(|d| d.get_temp::<Arc<Value>>(store()))
}

/// Replace the whole override set and re-apply the active preset under it. `null` or an empty
/// object clears it. Unknown and invalid keys are reported and left out; the rest still applies.
pub fn set(ctx: &egui::Context, overrides: &Value) -> Result<Report, String> {
    let (clean, report) = sort(overrides)?;
    put(ctx, (!clean.is_empty()).then(|| Arc::new(Value::Object(clean))));
    Ok(report)
}

/// Put back a set returned by [`current`] (or none) and re-apply the active preset under it.
pub fn put(ctx: &egui::Context, overrides: Option<Arc<Value>>) {
    ctx.data_mut(|d| {
        d.remove::<Arc<Value>>(store());
        if let Some(overrides) = overrides {
            d.insert_temp(store(), overrides);
        }
        // The canvas surround caches its dot texture per preset (`canvas::dots`), not per colour.
        for kind in ThemeKind::ALL {
            d.remove::<egui::TextureHandle>(egui::Id::new(("canvas-dots", format!("{kind:?}"))));
        }
    });
    // Before the first `apply` there is no preset to redo; that `apply` will pick the set up.
    if let Some(active) = ctx.data(|d| d.get_temp::<Tokens>(egui::Id::new("photocraft-theme"))) {
        super::apply(ctx, active.kind);
    }
    ctx.request_repaint();
}

/// `t` with the host's overrides laid over it. Called by [`super::apply`] for every preset.
pub(super) fn resolve(ctx: &egui::Context, mut t: Tokens) -> Tokens {
    if let Some(overrides) = current(ctx) {
        super::live::apply_overrides(&mut t, &overrides);
    }
    t
}

fn sort(overrides: &Value) -> Result<(Map<String, Value>, Report), String> {
    let entries = match overrides {
        Value::Null => return Ok(Default::default()),
        Value::Object(entries) => entries,
        _ => return Err(SHAPE.into()),
    };
    // Upstream's own list decides which keys are tokens, so a pull that renames one is reported.
    let mut scratch = Tokens::for_kind(ThemeKind::default());
    let mut report = Report { unknown: super::live::apply_overrides(&mut scratch, overrides), ..Default::default() };
    let mut clean = Map::new();
    for (key, value) in entries {
        if report.unknown.contains(key) {
            continue;
        }
        let valid = if RADII.contains(&key.as_str()) { value.as_f64().is_some_and(|r| (0.0..=255.0).contains(&r)) } else { is_colour(value) };
        if valid {
            clean.insert(key.clone(), value.clone());
            report.applied.push(key.clone());
        } else {
            report.invalid.push(key.clone());
        }
    }
    Ok((clean, report))
}

fn is_colour(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        let hex = text.strip_prefix('#').unwrap_or(text);
        matches!(hex.len(), 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhotocraftApp;
    use crate::control::{ControlRequest, Outcome, handle};
    use egui::{Color32, CornerRadius};

    const CARD: Color32 = Color32::from_rgb(16, 32, 48);
    const FIELD: Color32 = Color32::from_rgb(10, 11, 12);
    const ACCENT: Color32 = Color32::from_rgb(200, 30, 90);

    fn palette() -> Value {
        json!({"card": "#102030", "field": "#0a0b0c", "accent": "#c81e5a", "accent_soft": "#c81e5a", "accent_border": "c81e5a", "radius_sm": 2})
    }

    /// The host's values are in the tokens and in egui's own visuals, in both of egui's style
    /// branches; what the host did not set is the preset's.
    fn assert_palette(ctx: &egui::Context, kind: ThemeKind, when: &str) {
        let preset = Tokens::for_kind(kind);
        let t = Tokens::get(ctx);
        assert_eq!((t.kind, t.card, t.field, t.accent, t.radius_sm), (kind, CARD, FIELD, ACCENT, 2.0), "{when}: tokens");
        assert_eq!((t.text, t.chrome, t.radius_lg), (preset.text, preset.chrome, preset.radius_lg), "{when}: the rest is {kind:?}");
        for style in [ctx.global_style(), ctx.style_of(egui::Theme::Light), ctx.style_of(egui::Theme::Dark)] {
            let v = &style.visuals;
            assert_eq!((v.window_fill, v.faint_bg_color, v.widgets.noninteractive.bg_fill), (CARD, CARD, CARD), "{when}: card visuals");
            assert_eq!((v.extreme_bg_color, v.widgets.inactive.bg_fill, v.widgets.inactive.weak_bg_fill), (FIELD, FIELD, FIELD), "{when}: field visuals");
            assert_eq!((v.hyperlink_color, v.selection.bg_fill), (ACCENT, ACCENT), "{when}: accent visuals");
            // The bevelled preset (Classic) outlines its widgets in a fixed grey.
            assert_eq!(v.widgets.active.bg_stroke.color, if preset.bevel { Color32::from_gray(64) } else { ACCENT }, "{when}: pressed widget outline");
            assert_eq!(v.widgets.inactive.corner_radius, CornerRadius::same(2), "{when}: widget radius");
            assert_eq!(v.panel_fill, preset.chrome, "{when}: panel fill is the preset's");
        }
    }

    #[test]
    fn host_palette_applies_to_tokens_and_widget_visuals_and_reports_each_key() {
        let ctx = egui::Context::default();
        super::super::apply(&ctx, ThemeKind::StudioLight);
        let mut sent = palette();
        sent["nope"] = json!(1);
        sent["card_colour"] = json!("#ffffff");
        sent["hover"] = json!("red");
        sent["radius"] = json!("4");
        let report = set(&ctx, &sent).unwrap();
        assert_eq!(report.applied, ["accent", "accent_border", "accent_soft", "card", "field", "radius_sm"]);
        assert_eq!(report.unknown, ["card_colour", "nope"], "a renamed token is reported, not dropped silently");
        assert_eq!(report.invalid, ["hover", "radius"]);
        assert_eq!(report.to_json(), json!({"applied": report.applied, "unknown": ["card_colour", "nope"], "invalid": ["hover", "radius"]}));
        assert_palette(&ctx, ThemeKind::StudioLight, "set");
        let (t, preset) = (Tokens::get(&ctx), Tokens::for_kind(ThemeKind::StudioLight));
        assert_eq!((t.hover, t.radius), (preset.hover, preset.radius), "invalid values keep the preset's");
        assert_eq!(current(&ctx).as_deref(), Some(&palette()), "only the keys that apply are kept");
    }

    #[test]
    fn host_palette_survives_preset_changes_new_documents_and_frames() {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        let ctx = egui::Context::default();
        app.set_theme(&ctx, ThemeKind::StudioLight);
        set(&ctx, &palette()).unwrap();
        // Bevelled and Pro presets select with the accent itself; the palette sets both to one colour.
        for kind in ThemeKind::ALL {
            app.set_theme(&ctx, kind);
            assert_eq!(app.ui.theme, kind);
            assert_palette(&ctx, kind, "set_theme");
        }
        // The control protocol's own preset switch, as the host sends it.
        for (name, kind) in [("pro", ThemeKind::Pro), ("studioLight", ThemeKind::StudioLight)] {
            let (request, _reply) = ControlRequest::new("ui.set", json!({"theme": name}));
            let Outcome::Done(reply) = handle(&mut app, &ctx, &request) else { panic!("ui.set answers at once") };
            assert_eq!(reply["ok"], true, "{reply}");
            assert_palette(&ctx, kind, "ui.set theme");
        }
        app.run("file.new", json!({"width": 16, "height": 16})).unwrap();
        assert_palette(&ctx, ThemeKind::StudioLight, "new document");
        // Per-frame upkeep re-resolves the appearance; whatever preset it lands on carries the palette.
        for appearance in [Some(egui::Theme::Dark), Some(egui::Theme::Light), None, Some(egui::Theme::Dark)] {
            let input = egui::RawInput { system_theme: appearance, ..Default::default() };
            ctx.run_ui(input, |ui| crate::prefs_ui::tick(&mut app, ui.ctx())).textures_delta.clear();
            assert_palette(&ctx, app.ui.theme, "frame");
        }
        // Plain re-application, as the first frame and the preset previews do.
        super::super::apply(&ctx, ThemeKind::BreezeDark);
        assert_palette(&ctx, ThemeKind::BreezeDark, "apply");
    }

    #[test]
    fn host_palette_is_replaced_whole_cleared_and_restored() {
        let ctx = egui::Context::default();
        super::super::apply(&ctx, ThemeKind::Pro);
        let preset = Tokens::for_kind(ThemeKind::Pro);
        set(&ctx, &palette()).unwrap();
        let first = current(&ctx);
        // A new set replaces the old one: keys it leaves out go back to the preset.
        let report = set(&ctx, &json!({"text": "#01020380"})).unwrap();
        assert_eq!((report.applied, report.unknown, report.invalid), (vec!["text".to_string()], vec![], vec![]));
        let t = Tokens::get(&ctx);
        assert_eq!((t.text, t.card, t.radius_sm), (Color32::from_rgba_unmultiplied(1, 2, 3, 128), preset.card, preset.radius_sm));
        assert_eq!(ctx.global_style().visuals.override_text_color, Some(t.text));
        // A value of the wrong shape changes nothing.
        for bad in [json!([1]), json!("#102030"), json!(3), json!(true)] {
            assert!(set(&ctx, &bad).unwrap_err().contains("themeTokens must be an object"), "{bad}");
            assert_eq!(Tokens::get(&ctx), t, "{bad}");
        }
        // Putting back an earlier set (what the page bridge does when the rest of a request fails).
        put(&ctx, first);
        assert_palette(&ctx, ThemeKind::Pro, "put");
        // null and {} both clear: the preset is back, in the tokens and in egui's visuals.
        for clear in [Value::Null, json!({})] {
            set(&ctx, &palette()).unwrap();
            assert_eq!(set(&ctx, &clear), Ok(Report::default()));
            assert_eq!(current(&ctx), None);
            assert_eq!(Tokens::get(&ctx), preset);
            let fresh = egui::Context::default();
            super::super::apply(&fresh, ThemeKind::Pro);
            // (Two contexts' whole styles never compare equal: each has its own number formatter.)
            assert_eq!(ctx.global_style().visuals, fresh.global_style().visuals, "cleared: the visuals are the plain preset's again");
        }
        // Set before any preset was applied, the palette is picked up by the first `apply`.
        let early = egui::Context::default();
        set(&early, &palette()).unwrap();
        super::super::apply(&early, ThemeKind::Studio);
        assert_palette(&early, ThemeKind::Studio, "set before the first apply");
    }
}
