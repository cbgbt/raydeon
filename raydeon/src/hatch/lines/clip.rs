//! Interval arithmetic in a planar surface's own frame: clipping a line to
//! the convex outline, and cutting out the stretches where it crosses a
//! hole.
//!
//! A small interface on purpose — [`clip_to_outline`] and [`subtract_holes`]
//! are the only two entry points, shared verbatim by `hatch::lines::planar`
//! (axis-aligned hatch lines) and the contour engine's planar clip step
//! (`hatch::contour`, arbitrary marching-squares segments): one place owns
//! "what survives inside this outline minus its holes" for both callers.

use super::super::surface::FaceBox;
use super::FaceVec;

/// The range the outline spans along `direction`.
pub(super) fn extent_along(surface: &super::PlanarSurface, direction: FaceVec) -> (f64, f64) {
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
pub(crate) fn clip_to_outline(
    surface: &super::PlanarSurface,
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
pub(crate) fn subtract_holes(
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
