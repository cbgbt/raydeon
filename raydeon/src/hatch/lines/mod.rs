//! Laying hatch lines onto a surface, and keeping only the parts of them the
//! lighting calls for.
//!
//! Lines are generated in the surface's own frame, where clipping to the
//! outline and skipping holes is interval arithmetic (`clip`), then lifted
//! into world space for the renderer to occlude like any other geometry.

mod clip;
mod revolution;

pub(crate) use clip::{clip_to_outline, subtract_holes};

use super::surface::{PlanarSurface, RevolutionSurface, SphereSurface};
use super::HatchSpacing;
use crate::path::SlicedSegment3D;
use crate::ray::HitShape;
use crate::{DrawableShape, HitData, LineSegment3D, Scene, WPoint3, WVec3, WorldSpace};

/// A hatch line sits this far off its surface so that the renderer's
/// visibility ray does not immediately strike the surface the line is drawn
/// on and erase it.
///
/// `pub(crate)` because the contour grid (`hatch::contour::grid`) lifts its
/// nodes by the same amount: a `Tone` contour must sit at the position a
/// hatch chop measures its tone at (invariant 10), and both need the one
/// shared constant to do it.
pub(crate) const LINE_LIFT: f64 = 0.006;

/// Illumination samples are taken this far off the surface, for the same
/// reason: a shadow ray must clear the face it starts from.
const SAMPLE_LIFT: f64 = 0.005;

/// Distance between illumination samples along a hatch line, in world units.
///
/// The one const which owns "tone-field bandwidth": the contour engine's
/// default `ContourResolution` mints from this too, so retuning it cannot
/// silently diverge hatching and contour sampling.
pub(crate) const SAMPLE_LEN: f64 = 0.16;

/// Below this a parametric interval is too short to draw.
const MIN_INTERVAL: f64 = 1.0e-6;

/// A direction in a planar surface's frame.
pub(crate) type FaceVec = euclid::Vector2D<f64, super::surface::FaceSpace>;

/// Parallel lines across `surface` at `angle`, `spacing` apart, clipped to
/// the outline and interrupted by its holes.
pub(crate) fn planar(
    surface: &PlanarSurface,
    angle: euclid::Angle<f64>,
    spacing: HatchSpacing,
) -> Vec<LineSegment3D<WorldSpace>> {
    let spacing = spacing.into_inner();
    let dir = FaceVec::new(angle.radians.cos(), angle.radians.sin());
    let perp = FaceVec::new(-dir.y, dir.x);

    let (off_min, off_max) = clip::extent_along(surface, perp);
    let (t_min, t_max) = clip::extent_along(surface, dir);
    let lift = surface.normal() * LINE_LIFT;

    let mut segments = Vec::new();
    let mut offset = off_min + spacing / 2.0;
    while offset < off_max {
        let anchor = perp * offset;
        if let Some(span) = clip_to_outline(surface, anchor, dir, t_min, t_max) {
            for (start, end) in subtract_holes(surface.holes(), anchor, dir, span) {
                if end - start > MIN_INTERVAL {
                    let p1 = surface.to_world((anchor + dir * start).to_point()) + lift;
                    let p2 = surface.to_world((anchor + dir * end).to_point()) + lift;
                    segments.push(LineSegment3D::new_segment(p1, p2));
                }
            }
        }
        offset += spacing;
    }
    segments
}

/// Latitude rings around `axis`, spaced `spacing` apart along the surface,
/// emitted as chords short enough to be shaded individually.
pub(crate) fn sphere_rings(
    surface: &SphereSurface,
    axis: WVec3,
    spacing: HatchSpacing,
) -> Vec<LineSegment3D<WorldSpace>> {
    let spacing = spacing.into_inner();
    let axis = axis.normalize();
    if !axis.square_length().is_finite() || axis.square_length() < 0.5 {
        return Vec::new();
    }
    let (u, v) = ring_frame(axis);
    let radius = surface.radius() + LINE_LIFT;

    let mut segments = Vec::new();
    let steps = (std::f64::consts::PI * radius / spacing).floor() as usize;
    for step in 1..steps {
        let polar = step as f64 / steps as f64 * std::f64::consts::PI;
        let ring_radius = radius * polar.sin();
        let ring_center = surface.center() + axis * (radius * polar.cos());
        let chords = ((ring_radius * std::f64::consts::TAU) / SAMPLE_LEN).ceil() as usize;
        if chords < 8 {
            continue;
        }
        let at = |ndx: usize| {
            let angle = (ndx % chords) as f64 / chords as f64 * std::f64::consts::TAU;
            ring_center + u * (ring_radius * angle.cos()) + v * (ring_radius * angle.sin())
        };
        segments.extend((0..chords).map(|ndx| LineSegment3D::new_segment(at(ndx), at(ndx + 1))));
    }
    segments
}

pub(crate) use revolution::{revolution_meridians, revolution_rings};

/// Keeps the stretches of `segment` whose illumination passes `keep`,
/// rejoining stretches separated by a single rejected sample so that shading
/// reads as strokes rather than dashes.
///
/// `drawable` must be the scene's own entry for the surface being hatched:
/// the illumination query recognizes it and does not shadow the line against
/// the face it lies on.
pub(crate) fn filter_by_tone(
    scene: &Scene,
    drawable: &DrawableShape,
    eye: WPoint3,
    segment: &LineSegment3D<WorldSpace>,
    normal_at: impl Fn(WPoint3) -> WVec3,
    keep: impl Fn(f64, WPoint3) -> bool,
) -> Vec<LineSegment3D<WorldSpace>> {
    let num_chops = (segment.length() / SAMPLE_LEN).ceil() as usize;
    if num_chops == 0 {
        return Vec::new();
    }

    let mut sliced = SlicedSegment3D::new(num_chops, segment);
    let removals: Vec<usize> = sliced
        .subsegments()
        .enumerate()
        .filter_map(|(ndx, sub)| {
            let midpoint = sub.midpoint();
            let normal = normal_at(midpoint);
            let tone = surface_tone(scene, drawable, eye, midpoint, normal);
            (!keep(tone, midpoint)).then_some(ndx)
        })
        .collect();
    removals
        .into_iter()
        .for_each(|ndx| sliced.remove_subsegment(ndx));

    sliced.join_slices_with_forgiveness(1)
}

/// The tone (`[0, 1]`, scene-normalized brightness) at `point`, which must
/// already sit `LINE_LIFT` off the surface along `normal` — a hatch chop's
/// midpoint on a lifted line, or a contour grid node's own lifted position.
///
/// Adds `SAMPLE_LIFT` on top before casting the shadow ray, exactly as
/// `filter_by_tone` always has, so both callers measure the SAME field at
/// the SAME net lift (`LINE_LIFT + SAMPLE_LIFT`): a `Tone` contour therefore
/// lies precisely where a threshold-matched hatch pass starts drawing
/// (invariant 10), not merely close to it.
///
/// `drawable` must be the scene's own entry for the surface being sampled,
/// so the shadow query recognizes it and does not shadow the point against
/// the face it lies on.
pub(crate) fn surface_tone(
    scene: &Scene,
    drawable: &DrawableShape,
    eye: WPoint3,
    point: WPoint3,
    normal: WVec3,
) -> f64 {
    let hit = HitData::new(point + normal * SAMPLE_LIFT, 1.0, normal);
    scene.tone_for_hit(HitShape::new(hit, drawable), eye)
}

/// A pair of unit vectors spanning the plane perpendicular to `axis`.
///
/// Shared with [`crate::hatch::surface::RevolutionSurface`], which needs the
/// same reference frame to turn an angle into a radial direction: one basis
/// for "angle around this axis" everywhere it is asked for.
pub(crate) fn ring_frame(axis: WVec3) -> (WVec3, WVec3) {
    let seed = if axis.x.abs() < 0.9 {
        WVec3::new(1.0, 0.0, 0.0)
    } else {
        WVec3::new(0.0, 1.0, 0.0)
    };
    let u = axis.cross(seed).normalize();
    (u, axis.cross(u))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hatch::style::HatchSpacing;
    use crate::hatch::surface::{FaceBox, FacePoint, ProfilePoint};
    use proptest::prelude::*;

    fn spacing(value: f64) -> HatchSpacing {
        HatchSpacing::try_new(value).expect("test spacings are positive")
    }

    /// The unit-square face of the xy plane, with the given holes.
    fn square(holes: Vec<FaceBox>) -> PlanarSurface {
        PlanarSurface::try_new(
            WPoint3::new(0.0, 0.0, 0.0),
            [WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)],
            vec![
                FacePoint::new(0.0, 0.0),
                FacePoint::new(4.0, 0.0),
                FacePoint::new(4.0, 4.0),
                FacePoint::new(0.0, 4.0),
            ],
            holes,
        )
        .expect("a square is a surface")
    }

    fn hole(min: (f64, f64), max: (f64, f64)) -> FaceBox {
        FaceBox::new(FacePoint::new(min.0, min.1), FacePoint::new(max.0, max.1))
    }

    fn along_x() -> (FaceVec, FaceVec) {
        (FaceVec::new(1.0, 0.0), FaceVec::new(0.0, 1.0))
    }

    #[test]
    fn clipping_finds_the_stretch_of_a_line_inside_the_outline() {
        let (dir, perp) = along_x();
        let span = clip_to_outline(&square(vec![]), perp * 2.0, dir, 0.0, 4.0)
            .expect("a line through the middle crosses the square");
        assert_eq!(span, (0.0, 4.0));
    }

    #[test]
    fn a_line_outside_the_outline_is_clipped_away() {
        let (dir, perp) = along_x();
        assert!(clip_to_outline(&square(vec![]), perp * 9.0, dir, 0.0, 4.0).is_none());
    }

    #[test]
    fn a_diagonal_line_is_clipped_to_the_corner_it_crosses() {
        let surface = square(vec![]);
        let diagonal = FaceVec::new(1.0, 1.0).normalize();
        let perp = FaceVec::new(-diagonal.y, diagonal.x);
        // Offset off the main diagonal, so the line cuts across one corner.
        let offset = 2.0;
        let span = clip_to_outline(&surface, perp * offset, diagonal, -6.0, 6.0)
            .expect("the line crosses the corner");

        // A 45 degree line reaches the square's far corner at an offset of
        // 4 / sqrt(2), and shortens by two units of chord per unit of offset.
        let expected = 2.0 * (4.0 / 2.0f64.sqrt() - offset);
        let length = span.1 - span.0;
        assert!(
            (length - expected).abs() < 1.0e-9,
            "expected a chord of {expected}, got {length}"
        );
    }

    #[test]
    fn a_hole_interrupts_the_line_which_crosses_it() {
        let (dir, perp) = along_x();
        let holes = [hole((1.0, 0.5), (2.0, 3.5))];
        assert_eq!(
            subtract_holes(&holes, perp * 2.0, dir, (0.0, 4.0)),
            vec![(0.0, 1.0), (2.0, 4.0)]
        );
    }

    #[test]
    fn a_hole_the_line_misses_leaves_it_whole() {
        let (dir, perp) = along_x();
        let holes = [hole((1.0, 0.5), (2.0, 1.5))];
        assert_eq!(
            subtract_holes(&holes, perp * 3.0, dir, (0.0, 4.0)),
            vec![(0.0, 4.0)]
        );
    }

    #[test]
    fn overlapping_holes_cut_one_opening() {
        let (dir, perp) = along_x();
        let holes = [
            hole((1.0, 0.0), (2.5, 4.0)),
            hole((2.0, 0.0), (3.0, 4.0)),
            hole((1.2, 0.0), (1.4, 4.0)),
        ];
        assert_eq!(
            subtract_holes(&holes, perp * 2.0, dir, (0.0, 4.0)),
            vec![(0.0, 1.0), (3.0, 4.0)]
        );
    }

    #[test]
    fn a_hole_covering_the_whole_span_leaves_nothing() {
        let (dir, perp) = along_x();
        let holes = [hole((-1.0, -1.0), (5.0, 5.0))];
        assert!(subtract_holes(&holes, perp * 2.0, dir, (0.0, 4.0)).is_empty());
    }

    #[test]
    fn a_hatch_chop_and_a_contour_grid_node_at_the_same_point_measure_bit_identical_tone() {
        use crate::hatch::contour::grid;
        use crate::hatch::contour::ContourResolution;
        use crate::hatch::surface::HatchSurface;
        use crate::lights::PointLight;
        use crate::{DrawableShape, Material, PenId, Scene, SceneLighting, ToneWhite};
        use std::sync::Arc;

        // A resolution which divides the surface's 4x4 extent evenly, so a
        // grid node lands exactly at face point (2, 2) — the same point a
        // hatch chop can be made to sample.
        let surface = square(vec![]);
        let resolution = ContourResolution::try_new(1.0).expect("1.0 is a valid resolution");
        let grid = grid::surface_grid(&HatchSurface::Planar(surface.clone()), resolution);
        let (node_point, node_normal) = grid.node(2, 2);

        assert_eq!(
            node_point,
            surface.to_world(FacePoint::new(2.0, 2.0)) + surface.normal() * LINE_LIFT,
            "the grid node must carry the same LINE_LIFT a hatch line's chop midpoint does"
        );

        let material = Material::new().diffuse(1.0).pen(PenId::new(0)).build();
        let drawable = DrawableShape::new()
            .geometry(Arc::new(
                crate::shapes::Quad::new()
                    .origin(surface.to_world(FacePoint::new(0.0, 0.0)))
                    .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)])
                    .dims([4.0, 4.0])
                    .build(),
            ))
            .material(material)
            .build();
        let scene = Scene::new()
            .geometry(vec![drawable.clone()])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((1.0, 1.0, 5.0))
                            .intensity(2.0)
                            .build(),
                    )])
                    .with_tone_white(ToneWhite::try_new(1.0).expect("1.0 is a valid tone white")),
            )
            .build();
        let eye = WPoint3::new(2.0, 2.0, 10.0);

        // The hatch path: `filter_by_tone` samples at a chop's midpoint,
        // which is exactly this LINE_LIFT-lifted surface point when the
        // chop happens to land there.
        let hatch_sample_point =
            surface.to_world(FacePoint::new(2.0, 2.0)) + surface.normal() * LINE_LIFT;

        let hatch_tone = surface_tone(&scene, &drawable, eye, hatch_sample_point, surface.normal());
        let contour_tone = surface_tone(&scene, &drawable, eye, node_point, node_normal);

        assert_eq!(
            hatch_tone, contour_tone,
            "the same LINE_LIFT-lifted point must measure bit-identical tone from either caller"
        );
    }

    #[test]
    fn hatch_lines_skip_the_hole_they_cross() {
        let surface = square(vec![hole((1.0, 1.0), (3.0, 3.0))]);
        let lines = planar(&surface, euclid::Angle::degrees(0.0), spacing(0.5));

        assert!(!lines.is_empty(), "the square should carry hatch lines");
        for line in &lines {
            let (p1, p2) = (line.p1(), line.p2());
            assert!(
                p1.y <= 1.0 || p1.y >= 3.0 || p2.x <= 1.0 || p1.x >= 3.0,
                "a line crossed the hole: {p1:?} -> {p2:?}"
            );
        }
    }

    #[test]
    fn hatch_lines_are_lifted_off_their_surface() {
        let lines = planar(&square(vec![]), euclid::Angle::degrees(0.0), spacing(0.5));
        for line in lines {
            assert_eq!(line.p1().z, LINE_LIFT);
        }
    }

    #[test]
    fn tighter_spacing_draws_more_lines() {
        let surface = square(vec![]);
        let sparse = planar(&surface, euclid::Angle::degrees(30.0), spacing(1.0));
        let dense = planar(&surface, euclid::Angle::degrees(30.0), spacing(0.25));
        assert!(dense.len() > sparse.len());
    }

    #[test]
    fn sphere_rings_stay_on_the_lifted_sphere() {
        let surface =
            SphereSurface::try_new(WPoint3::new(1.0, 2.0, 3.0), 2.0).expect("a real sphere");
        let rings = sphere_rings(&surface, WVec3::new(0.0, 0.0, 1.0), spacing(0.3));

        assert!(!rings.is_empty(), "a sphere should carry rings");
        for ring in rings {
            let offset = (ring.p1() - surface.center()).length();
            assert!(
                (offset - (surface.radius() + LINE_LIFT)).abs() < 1.0e-9,
                "ring point sits at {offset} from the center"
            );
        }
    }

    /// A vertical-wall (cylindrical) profile, so a ring's radial distance
    /// from the axis is exactly the profile's constant radius.
    fn cylinder(radius: f64) -> RevolutionSurface {
        RevolutionSurface::try_new(
            WPoint3::new(1.0, 2.0, 3.0),
            WVec3::new(0.0, 0.0, 1.0),
            vec![
                ProfilePoint {
                    radius,
                    height: 0.0,
                },
                ProfilePoint {
                    radius,
                    height: 2.0,
                },
            ],
        )
        .expect("a straight wall is a surface")
    }

    #[test]
    fn revolution_rings_stay_on_the_lifted_surface() {
        let surface = cylinder(1.5);
        let rings = revolution_rings(&surface, spacing(0.3));

        assert!(!rings.is_empty(), "a cylinder should carry rings");
        for ring in rings {
            let offset = ring.p1() - surface.base();
            let height = offset.dot(surface.axis());
            let radius = (offset - surface.axis() * height).length();
            assert!(
                (radius - (1.5 + LINE_LIFT)).abs() < 1.0e-9,
                "ring point sits at radius {radius} from the axis"
            );
        }
    }

    #[test]
    fn revolution_meridians_span_the_whole_profile() {
        let surface = cylinder(1.0);
        let meridians = revolution_meridians(&surface, spacing(0.4));

        assert!(!meridians.is_empty(), "a cylinder should carry meridians");
        let heights: Vec<f64> = meridians
            .iter()
            .flat_map(|line| {
                let base = surface.base();
                let axis = surface.axis();
                [(line.p1() - base).dot(axis), (line.p2() - base).dot(axis)]
            })
            .collect();
        let min_height = heights.iter().copied().fold(f64::MAX, f64::min);
        let max_height = heights.iter().copied().fold(f64::MIN, f64::max);

        assert!(
            min_height < SAMPLE_LEN,
            "meridians should reach the foot, min height was {min_height}"
        );
        assert!(
            max_height > 2.0 - SAMPLE_LEN,
            "meridians should reach the lip, max height was {max_height}"
        );
    }

    proptest! {
        /// Whatever the holes, what survives lies inside the original span,
        /// comes out in ascending order, and never overlaps a hole.
        #[test]
        fn subtraction_leaves_ordered_stretches_clear_of_every_hole(
            holes in proptest::collection::vec((0.0f64..4.0, 0.1f64..3.0), 0..5),
        ) {
            let (dir, perp) = along_x();
            let boxes: Vec<FaceBox> = holes
                .iter()
                .map(|&(start, width)| hole((start, 0.0), (start + width, 4.0)))
                .collect();
            let remaining = subtract_holes(&boxes, perp * 2.0, dir, (0.0, 4.0));

            let mut previous_end = f64::NEG_INFINITY;
            for &(start, end) in remaining.iter() {
                prop_assert!(start >= 0.0 && end <= 4.0, "({start}, {end}) escaped the span");
                prop_assert!(start < end, "({start}, {end}) is not a stretch");
                prop_assert!(start >= previous_end, "stretches out of order");
                previous_end = end;

                let midpoint = (start + end) / 2.0;
                for &(hole_start, width) in holes.iter() {
                    prop_assert!(
                        midpoint <= hole_start || midpoint >= hole_start + width,
                        "{midpoint} lies inside a hole",
                    );
                }
            }
        }

        /// A single hole removes exactly its own width from the span.
        #[test]
        fn one_hole_removes_exactly_its_width(start in 0.5f64..2.0, width in 0.1f64..1.0) {
            let (dir, perp) = along_x();
            let boxes = [hole((start, 0.0), (start + width, 4.0))];
            let remaining = subtract_holes(&boxes, perp * 2.0, dir, (0.0, 4.0));
            let total: f64 = remaining.iter().map(|(a, b)| b - a).sum();
            prop_assert!((total - (4.0 - width)).abs() < 1.0e-9, "kept {total}");
        }
    }
}
