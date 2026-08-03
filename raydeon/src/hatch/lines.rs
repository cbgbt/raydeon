//! Laying hatch lines onto a surface, and keeping only the parts of them the
//! lighting calls for.
//!
//! Lines are generated in the surface's own frame, where clipping to the
//! outline and skipping holes is interval arithmetic, then lifted into world
//! space for the renderer to occlude like any other geometry.

use super::surface::{FaceBox, PlanarSurface, SphereSurface};
use super::HatchSpacing;
use crate::path::SlicedSegment3D;
use crate::ray::HitShape;
use crate::{DrawableShape, HitData, LineSegment3D, Scene, WPoint3, WVec3, WorldSpace};

/// A hatch line sits this far off its surface so that the renderer's
/// visibility ray does not immediately strike the surface the line is drawn
/// on and erase it.
const LINE_LIFT: f64 = 0.006;

/// Illumination samples are taken this far off the surface, for the same
/// reason: a shadow ray must clear the face it starts from.
const SAMPLE_LIFT: f64 = 0.005;

/// Distance between illumination samples along a hatch line, in world units.
const SAMPLE_LEN: f64 = 0.16;

/// Below this a parametric interval is too short to draw.
const MIN_INTERVAL: f64 = 1.0e-6;

/// A direction in a planar surface's frame.
type FaceVec = euclid::Vector2D<f64, super::surface::FaceSpace>;

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

    let (off_min, off_max) = extent_along(surface, perp);
    let (t_min, t_max) = extent_along(surface, dir);
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
            let hit = HitData::new(midpoint + normal * SAMPLE_LIFT, 1.0, normal);
            let tone = scene.tone_for_hit(HitShape::new(hit, drawable), eye);
            (!keep(tone, midpoint)).then_some(ndx)
        })
        .collect();
    removals
        .into_iter()
        .for_each(|ndx| sliced.remove_subsegment(ndx));

    sliced.join_slices_with_forgiveness(1)
}

/// A pair of unit vectors spanning the plane perpendicular to `axis`.
fn ring_frame(axis: WVec3) -> (WVec3, WVec3) {
    let seed = if axis.x.abs() < 0.9 {
        WVec3::new(1.0, 0.0, 0.0)
    } else {
        WVec3::new(0.0, 1.0, 0.0)
    };
    let u = axis.cross(seed).normalize();
    (u, axis.cross(u))
}

/// The range the outline spans along `direction`.
fn extent_along(surface: &PlanarSurface, direction: FaceVec) -> (f64, f64) {
    surface
        .outline()
        .iter()
        .map(|corner| corner.to_vector().dot(direction))
        .fold((f64::MAX, f64::MIN), |(lo, hi), value| {
            (lo.min(value), hi.max(value))
        })
}

/// Clips the line `anchor + t * dir` to the convex outline, returning the
/// range of `t` inside it.
fn clip_to_outline(
    surface: &PlanarSurface,
    anchor: FaceVec,
    dir: FaceVec,
    t_min: f64,
    t_max: f64,
) -> Option<(f64, f64)> {
    let outline = surface.outline();
    let mut lo = t_min - 1.0;
    let mut hi = t_max + 1.0;
    let corners = outline.len();

    for ndx in 0..corners {
        let a = outline[ndx];
        let b = outline[(ndx + 1) % corners];
        // Inward normal of the edge, for a counter-clockwise outline.
        let edge = b - a;
        let inward = FaceVec::new(-edge.y, edge.x);

        let denom = inward.dot(dir);
        let dist = inward.dot(a.to_vector() - anchor);
        if denom.abs() < 1.0e-12 {
            // Parallel to the edge: either wholly inside it or wholly outside.
            if dist > 0.0 {
                return None;
            }
            continue;
        }
        let t = dist / denom;
        if denom > 0.0 {
            lo = lo.max(t);
        } else {
            hi = hi.min(t);
        }
    }

    (hi - lo > 1.0e-9).then_some((lo, hi))
}

/// Splits `span` around the stretches where the line passes through a hole.
fn subtract_holes(
    holes: &[FaceBox],
    anchor: FaceVec,
    dir: FaceVec,
    span: (f64, f64),
) -> Vec<(f64, f64)> {
    let mut cuts: Vec<(f64, f64)> = holes
        .iter()
        .filter_map(|hole| hole_span(hole, anchor, dir))
        .collect();
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut remaining = Vec::new();
    let mut cursor = span.0;
    for (cut_lo, cut_hi) in cuts {
        if cut_hi < cursor || cut_lo > span.1 {
            continue;
        }
        if cut_lo > cursor {
            remaining.push((cursor, cut_lo));
        }
        cursor = cursor.max(cut_hi);
    }
    if cursor < span.1 {
        remaining.push((cursor, span.1));
    }
    remaining
}

/// The range of `t` for which `anchor + t * dir` lies within `hole`, by slab
/// intersection on each axis.
fn hole_span(hole: &FaceBox, anchor: FaceVec, dir: FaceVec) -> Option<(f64, f64)> {
    let axes = [
        (anchor.x, dir.x, hole.min.x, hole.max.x),
        (anchor.y, dir.y, hole.min.y, hole.max.y),
    ];

    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    for (origin, direction, slab_lo, slab_hi) in axes {
        if direction.abs() < 1.0e-12 {
            if origin < slab_lo || origin > slab_hi {
                return None;
            }
            continue;
        }
        let first = (slab_lo - origin) / direction;
        let second = (slab_hi - origin) / direction;
        lo = lo.max(first.min(second));
        hi = hi.min(first.max(second));
    }
    (hi > lo).then_some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hatch::style::HatchSpacing;
    use crate::hatch::surface::FacePoint;
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
