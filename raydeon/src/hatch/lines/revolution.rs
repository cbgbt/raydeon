//! Hatch line generation for [`RevolutionSurface`]: rings (latitude bands
//! stepped by profile arc length) and meridians (profile curves repeated
//! around the axis), both lifted off the surface along its own outward
//! normal so they survive their own occlusion check.

use super::{RevolutionSurface, LINE_LIFT, MIN_INTERVAL, SAMPLE_LEN};
use crate::hatch::HatchSpacing;
use crate::{LineSegment3D, WPoint3, WorldSpace};

/// Rings around the axis, one every `spacing` of profile arc length,
/// emitted as chords short enough to be shaded individually and lifted off
/// the surface along its own outward normal.
pub(crate) fn revolution_rings(
    surface: &RevolutionSurface,
    spacing: HatchSpacing,
) -> Vec<LineSegment3D<WorldSpace>> {
    let spacing = spacing.into_inner();
    let arc_length = surface.arc_length();
    if arc_length < MIN_INTERVAL {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut t = spacing / 2.0;
    while t < arc_length {
        segments.extend(revolution_ring_at(surface, t));
        t += spacing;
    }
    segments
}

/// The chords of one ring at profile arc length `t`, or none if the ring is
/// too small a circle to draw usefully.
fn revolution_ring_at(surface: &RevolutionSurface, t: f64) -> Vec<LineSegment3D<WorldSpace>> {
    let radius = revolution_radius_at(surface, t);
    let chords = ((radius * std::f64::consts::TAU) / SAMPLE_LEN).ceil() as usize;
    if chords < 8 {
        return Vec::new();
    }
    let at = |ndx: usize| {
        let angle =
            euclid::Angle::radians((ndx % chords) as f64 / chords as f64 * std::f64::consts::TAU);
        revolution_lifted_point(surface, t, angle)
    };
    (0..chords)
        .map(|ndx| LineSegment3D::new_segment(at(ndx), at(ndx + 1)))
        .collect()
}

/// Meridians: profile curves repeated every `spacing` of arc length around
/// the widest ring, each subdivided at [`SAMPLE_LEN`] along the profile so
/// shading can chop them individually.
pub(crate) fn revolution_meridians(
    surface: &RevolutionSurface,
    spacing: HatchSpacing,
) -> Vec<LineSegment3D<WorldSpace>> {
    let spacing = spacing.into_inner();
    let max_radius = surface.max_radius();
    let arc_length = surface.arc_length();
    if max_radius < MIN_INTERVAL || arc_length < MIN_INTERVAL {
        return Vec::new();
    }

    let steps = ((max_radius * std::f64::consts::TAU) / spacing).floor() as usize;
    if steps == 0 {
        return Vec::new();
    }
    let samples = ((arc_length / SAMPLE_LEN).ceil() as usize).max(1);

    let mut segments = Vec::new();
    for step in 0..steps {
        let angle = euclid::Angle::radians(step as f64 / steps as f64 * std::f64::consts::TAU);
        let at = |sample: usize| {
            let t = sample as f64 / samples as f64 * arc_length;
            revolution_lifted_point(surface, t, angle)
        };
        segments.extend(
            (0..samples).map(|sample| LineSegment3D::new_segment(at(sample), at(sample + 1))),
        );
    }
    segments
}

/// The perpendicular distance from the axis at profile arc length `t`.
fn revolution_radius_at(surface: &RevolutionSurface, t: f64) -> f64 {
    let point = surface.point_at(t, euclid::Angle::radians(0.0));
    let offset = point - surface.base();
    let along_axis = offset.dot(surface.axis());
    (offset - surface.axis() * along_axis).length()
}

/// The world position at profile arc length `t` and `angle`, lifted off the
/// surface along its own outward normal so the drawn arc survives its own
/// occlusion check.
fn revolution_lifted_point(
    surface: &RevolutionSurface,
    t: f64,
    angle: euclid::Angle<f64>,
) -> WPoint3 {
    surface.point_at(t, angle) + surface.normal_at(t, angle) * LINE_LIFT
}
