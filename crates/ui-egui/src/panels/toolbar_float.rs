//! The toolbar as a palette over the canvas: a third mode after one and two columns
//! (`Panels.toolbar_floating`).
//!
//! [`super::toolbar`] draws the same tools either way; this module only swaps what they are drawn
//! in. Docked, that is upstream's left panel. Floating, it is an area above the canvas with a grip
//! to drag it by and a pin that locks it in place; nothing is taken from the frame, so the canvas
//! keeps the full width. The palette's corner is remembered in `Panels.toolbar_float` (saved with
//! the workspace like the other panel switches) as an offset from the canvas area's corner, and is
//! kept inside the canvas area when that shrinks.

use egui::{Rect, Sense, Stroke, Vec2, vec2};
use serde::{Deserialize, Serialize};

use crate::state::Panels;
use crate::theme::Tokens;
use crate::{PhotocraftApp, icons};

/// Where the floating toolbar sits (`Panels.toolbar_float`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Place {
    /// Its top-left corner, in points from the canvas area's top-left corner. `None`: the default
    /// corner, [`DEFAULT_OFFSET`].
    pub pos: Option<[f32; 2]>,
    /// The grip does not move it.
    pub pinned: bool,
}

/// The palette's corner before it was ever dragged.
pub const DEFAULT_OFFSET: [f32; 2] = [12.0, 12.0];
/// The palette stays this far inside the canvas area.
const EDGE: f32 = 6.0;
/// Height of the grip strip.
const GRIP: f32 = 12.0;
/// Side of the pin button. Not a tool button's size, so the two are told apart.
const PIN: f32 = 18.0;
/// Frame margin above and below, as the docked toolbar's.
const MARGIN_Y: f32 = 8.0;
/// The grip's accessible name.
pub const GRIP_LABEL: &str = "Move Toolbar";
/// The pin's accessible name and tooltip.
pub const PIN_LABEL: &str = "Pin Toolbar";

/// What [`super::toolbar`] draws its tools in: upstream's panel, or the palette.
pub(super) struct Host {
    panel: egui::Panel,
    float: Option<Float>,
}

struct Float {
    canvas: Rect,
    place: Place,
    /// Outer width, the inner margin on both sides included.
    width: f32,
    double: bool,
    margin: i8,
    /// Height the tools and the footer need, so the palette is no taller than its content.
    content: f32,
}

/// Docked or floating, as `Panels.toolbar_floating` says. `metrics` are the toolbar's own: its
/// width, a tool button's side and the horizontal inner margin.
pub(super) fn host(app: &PhotocraftApp, ctx: &egui::Context, metrics: (f32, f32, i8), panel: egui::Panel) -> Host {
    let (width, button, margin) = metrics;
    let panels = &app.ui.panels;
    let float = panels.toolbar_floating.then(|| {
        let slots: Vec<usize> = super::visible_sections(&app.session.prefs().toolbar.hidden).iter().map(Vec::len).collect();
        let content = content_height(&Tokens::get(ctx), &slots, button, panels.toolbar_double);
        Float { canvas: app.last_canvas_rect, place: panels.toolbar_float, width, double: panels.toolbar_double, margin, content }
    });
    Host { panel, float }
}

/// The height [`super::toolbar`]'s contents take: the tool rows, the group dividers, Pro's
/// Edit Toolbar button and the footer (colour chips, Pro's Quick Mask and Screen Mode buttons).
/// Mirrors the spacing in `toolbar`; a few points of slack cover what it does not mirror, and a
/// shortfall only makes the tool column scroll.
fn content_height(t: &Tokens, slots: &[usize], button: f32, double: bool) -> f32 {
    let per_row = if double { 2 } else { 1 };
    // Pro has no group dividers, so its two columns fill every row across groups.
    let rows: usize = if t.pro { slots.iter().sum::<usize>().div_ceil(per_row) } else { slots.iter().map(|n| n.div_ceil(per_row)).sum() };
    let dividers = if t.pro { 0 } else { slots.len().saturating_sub(1) };
    let tools = rows as f32 * (button + 3.0) + dividers as f32 * 12.0 + if t.pro { button + 3.0 } else { 0.0 };
    let footer = (if t.pro { 8.0 } else { 14.0 })
        + super::chips_height(t.pro, t.round_chips, t.round_chips && double)
        + if t.pro { 8.0 + if double { button } else { button * 2.0 + 3.0 } } else { 0.0 }
        + 6.0;
    tools + footer + 4.0
}

/// A change the palette's own controls asked for: the new `toolbar_floating`, `toolbar_double`
/// and `toolbar_float`.
pub(super) type Change = Option<(bool, bool, Place)>;

/// The four toolbar states, as `(label, floating, double)`.
pub const MODES: [(&str, bool, bool); 4] = [
    ("Docked, one column", false, false),
    ("Docked, two columns", false, true),
    ("Floating, one column", true, false),
    ("Floating, two columns", true, true),
];

/// The four states as selectable items; returns the one picked.
fn mode_items(ui: &mut egui::Ui, now: (bool, bool)) -> Option<(bool, bool)> {
    let mut picked = None;
    for (label, floating, double) in MODES {
        if ui.add(egui::Button::selectable(now == (floating, double), crate::i18n::t(label))).clicked() {
            picked = Some((floating, double));
            ui.close();
        }
    }
    picked
}

impl Host {
    /// Draws `add_contents` in the panel or in the palette.
    pub(super) fn show(self, ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) -> Change {
        match self.float {
            Some(float) => float.show(ui.ctx(), add_contents),
            None => {
                self.panel.show(ui, add_contents);
                None
            }
        }
    }
}

/// Applies what the palette's controls asked for, once the tools are drawn.
pub(super) fn settle(panels: &mut Panels, change: Change) {
    if let Some((floating, double, place)) = change {
        panels.toolbar_floating = floating;
        panels.toolbar_double = double;
        panels.toolbar_float = place;
    }
}

/// The header chevron's context menu: the four toolbar states.
pub(super) fn mode_menu(chevron: &egui::Response, panels: &mut Panels) {
    egui::Popup::context_menu(chevron).show(|ui| {
        ui.set_min_width(160.0);
        if let Some(mode) = mode_items(ui, (panels.toolbar_floating, panels.toolbar_double)) {
            (panels.toolbar_floating, panels.toolbar_double) = mode;
        }
    });
}

/// The palette's corner for a wanted `offset` from the canvas corner, kept inside the canvas area.
pub fn clamp_offset(canvas: Rect, size: Vec2, offset: Vec2) -> Vec2 {
    let hi = (canvas.size() - size - Vec2::splat(EDGE)).max(Vec2::splat(EDGE));
    offset.clamp(Vec2::splat(EDGE), hi)
}

impl Float {
    fn show(self, ctx: &egui::Context, add_contents: impl FnOnce(&mut egui::Ui)) -> Change {
        let t = Tokens::get(ctx);
        let id = egui::Id::new("toolbar-float");
        // The canvas area is the last frame's (the toolbar is drawn before the canvas): when it
        // changed, as it does on the frame the toolbar leaves the frame, draw once more.
        if ctx.data(|d| d.get_temp::<Rect>(id.with("canvas"))) != Some(self.canvas) {
            ctx.data_mut(|d| d.insert_temp(id.with("canvas"), self.canvas));
            ctx.request_repaint();
        }
        let inner = self.width - 2.0 * f32::from(self.margin);
        let header = GRIP + 3.0 + PIN + 4.0;
        let chrome = 2.0 * MARGIN_Y + header + 2.0;
        let body = self.content.min(self.canvas.height() - 2.0 * EDGE - chrome).max(0.0);
        let size = vec2(self.width + 2.0, chrome + body);
        let offset = clamp_offset(self.canvas, size, Vec2::from(self.place.pos.unwrap_or(DEFAULT_OFFSET)));
        let frame = egui::Frame::NONE
            .fill(t.chrome)
            .stroke(Stroke::new(1.0, t.card_border))
            .corner_radius(egui::CornerRadius::same(t.radius as u8))
            .shadow(egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: t.shadow })
            .inner_margin(egui::Margin::symmetric(self.margin, MARGIN_Y as i8));
        let mut place = self.place;
        let mut mode = (true, self.double);
        // Order::Middle like the other floating panels, so menus, popups and dialogs stay above it.
        egui::Area::new(id).order(egui::Order::Middle).fixed_pos(self.canvas.min + offset).show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(inner);
                ui.spacing_mut().item_spacing = vec2(0.0, 3.0);
                let (strip, grip) = ui.allocate_exact_size(vec2(inner, GRIP), Sense::click_and_drag());
                grip.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, crate::i18n::t(GRIP_LABEL)));
                let tint = if grip.hovered() || grip.dragged() { t.text } else { t.text_faint };
                for column in -1..=1 {
                    for row in [-2.0, 2.0] {
                        ui.painter().circle_filled(strip.center() + vec2(column as f32 * 4.0, row), 1.0, tint);
                    }
                }
                if !place.pinned {
                    if grip.hovered() || grip.dragged() {
                        ctx.set_cursor_icon(if grip.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                    }
                    if grip.dragged() && grip.drag_delta() != Vec2::ZERO {
                        place.pos = Some(clamp_offset(self.canvas, size, offset + grip.drag_delta()).into());
                    }
                }
                egui::Popup::context_menu(&grip).show(|ui| {
                    ui.set_min_width(160.0);
                    if ui.add(egui::Button::selectable(place.pinned, crate::i18n::t(PIN_LABEL))).clicked() {
                        place.pinned = !place.pinned;
                        ui.close();
                    }
                    ui.separator();
                    if let Some(picked) = mode_items(ui, mode) {
                        mode = picked;
                    }
                });
                ui.vertical_centered(|ui| {
                    if icons::button(ui, if place.pinned { "lock" } else { "lock-open" }, PIN, place.pinned, PIN_LABEL).clicked() {
                        place.pinned = !place.pinned;
                    }
                });
                ui.add_space(4.0);
                ui.allocate_ui(vec2(inner, body), |ui| {
                    ui.set_min_width(inner);
                    add_contents(ui);
                });
            });
        });
        ((mode, place) != ((true, self.double), self.place)).then_some((mode.0, mode.1, place))
    }
}
