//! The marching ants' clock: the dash phase moves in whole physical pixels, at a fixed rate.
//!
//! This file is not upstream PhotoCraft's: it belongs to the C0VIBE host shell (see
//! `c0/photocraft-host/README.md`). Upstream took the phase straight from the frame time, so every
//! frame that was not on the 100 ms repaint grid (a pointer move, a request from the host, another
//! animation) drew the dashes a fraction of a pixel away from the last one and the outline shimmered.
//! Photoshop's ants move a pixel at a time, at a steady rate, whatever else is drawn. The phase is a
//! function of the tick (`floor(time * TICKS_PER_SECOND)`), so any two frames in one tick draw the
//! same dashes.

use crate::tool_feedback::DASH;

/// Steps per second: as upstream's `time * 10.0` points per second, in whole steps.
pub const TICKS_PER_SECOND: f64 = 10.0;

/// The tick `time` (seconds) is in.
pub fn tick(time: f64) -> u64 {
    (time.max(0.0) * TICKS_PER_SECOND).floor() as u64
}

/// Seconds from `time` to the start of the next tick (never zero, so a repaint asked for with it
/// lands in the next tick).
pub fn until_next_tick(time: f64) -> f64 {
    (((tick(time) + 1) as f64 / TICKS_PER_SECOND) - time).max(0.001)
}

/// The dash phase in screen points for frame time `time` on a display with `pixels_per_point`.
///
/// One tick moves the dashes by `round(pixels_per_point)` physical pixels (one point on a 1x or 2x
/// display), and the phase wraps at the dash period measured in physical pixels, so on every
/// display the phase is a whole number of physical pixels.
pub fn phase_at(time: f64, pixels_per_point: f32) -> f32 {
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 { pixels_per_point } else { 1.0 };
    let step_px = ppp.round().max(1.0) as u64;
    let period_px = ((DASH * 2.0 * ppp).round() as u64).max(1);
    let px = (tick(time) % period_px) * step_px % period_px;
    px as f32 / ppp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_is_the_same_for_every_frame_inside_one_tick() {
        for ppp in [1.0f32, 1.5, 2.0, 3.0] {
            for n in [0u64, 1, 37, 12345] {
                let start = (n as f64 + 0.01) / 10.0;
                let phase = phase_at(start, ppp);
                for dt in [0.0, 0.0001, 0.013, 0.033, 0.0499, 0.0789] {
                    assert_eq!(phase_at(start + dt, ppp), phase, "ppp {ppp} start {start} +{dt}");
                }
            }
        }
    }

    #[test]
    fn the_phase_advances_by_exactly_one_step_across_the_boundary() {
        // 1x: one point per tick.
        assert_eq!(phase_at(0.05, 1.0), 0.0);
        assert_eq!(phase_at(0.1001, 1.0), 1.0);
        assert_eq!(phase_at(0.2001, 1.0), 2.0);
        // 2x: two physical pixels (one point) per tick.
        assert_eq!(phase_at(0.1001, 2.0), 1.0);
        // 1.5x: round(1.5) = 2 pixels = 4/3 points.
        assert!((phase_at(0.1001, 1.5) - 2.0 / 1.5).abs() < 1e-6);
        for ppp in [1.0f32, 1.25, 1.5, 2.0] {
            let step = phase_at(0.1001, ppp) - phase_at(0.0, ppp);
            assert!((step - ppp.round().max(1.0) / ppp).abs() < 1e-6, "ppp {ppp}: {step}");
        }
    }

    #[test]
    fn the_phase_is_a_whole_number_of_physical_pixels_and_wraps_without_a_jump() {
        for ppp in [1.0f32, 1.5, 2.0, 3.0] {
            let period_px = (DASH * 2.0 * ppp).round() as u64;
            let mut seen = std::collections::BTreeSet::new();
            for tick in 0..200u64 {
                let phase = phase_at(tick as f64 / 10.0 + 0.01, ppp);
                let px = phase * ppp;
                assert!((px - px.round()).abs() < 1e-3, "ppp {ppp} tick {tick}: {px} is not a whole pixel");
                assert!(px.round() as u64 <= period_px, "ppp {ppp}: {px} past the period {period_px}");
                seen.insert(px.round() as u64);
            }
            assert!(seen.len() > 1, "ppp {ppp}: the phase never moved");
        }
        // Whole period at 1x: after 8 ticks the dashes are where they began.
        assert_eq!(phase_at(0.8001, 1.0), phase_at(0.0, 1.0));
    }

    #[test]
    fn the_dashes_of_a_step_are_identical_whatever_frames_were_drawn_in_between() {
        use egui::pos2;
        let pts = [pos2(3.0, 4.0), pos2(77.0, 4.0), pos2(77.0, 51.0), pos2(3.0, 51.0)];
        for ppp in [1.0f32, 2.0] {
            let at = |t: f64| crate::tool_feedback::ants(&pts, true, phase_at(t, ppp));
            // Frames at an arbitrary scatter of times inside tick 7 draw the same segments.
            let seven = at(0.701);
            for t in [0.7013, 0.7117, 0.7333, 0.7499, 0.7611, 0.7799] {
                assert!(at(t) == seven, "ppp {ppp}: the dashes at {t} differ from the step's");
            }
            // The next tick moves them, and moves them as far as one step.
            let eight = at(0.801);
            assert!(eight != seven, "ppp {ppp}: the ants did not march");
            let first = |shapes: &[egui::Shape]| {
                shapes.iter().find_map(|s| match s {
                    egui::Shape::LineSegment { points, .. } => Some(points[1]),
                    _ => None,
                })
            };
            let (a, b) = (first(&seven).unwrap(), first(&eight).unwrap());
            assert!((a - b).length() > 0.0 && (a - b).length() <= 2.0, "{a:?} {b:?}");
        }
    }

    #[test]
    fn the_next_tick_is_always_ahead() {
        for t in [0.0f64, 0.05, 0.1, 0.0999999, 7.3] {
            let wait = until_next_tick(t);
            assert!(wait > 0.0 && wait <= 0.1 + 1e-9, "{t}: {wait}");
            assert_eq!(tick(t + wait + 1e-6), tick(t) + 1, "{t}");
        }
    }

    #[test]
    fn bad_densities_do_not_panic() {
        for ppp in [0.0f32, -1.0, f32::NAN, f32::INFINITY] {
            assert!(phase_at(5.0, ppp).is_finite());
        }
    }
}
