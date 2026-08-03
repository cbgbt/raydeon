//! World-space hatch line generation and illumination-driven filtering.
//!
//! Hatch lines are generated in a face's own plane coordinates, clipped to
//! its convex outline minus any holes, then filtered sample-by-sample against
//! scene illumination so shading density follows the light.
use crate::scene::{Ball, Face, TestScene, TONE_WHITE};
use raydeon::path::SlicedSegment3D;
use raydeon::ray::HitShape;
use raydeon::{HitData, LineSegment3D, Scene, WPoint3, WVec3, WorldSpace};

/// Distance between illumination samples along a hatch line, in world units.
const SAMPLE_LEN: f64 = 0.16;
/// Samples sit this far off the surface so shadow rays see their own face.
const SURFACE_LIFT: f64 = 0.005;

/// Perceived surface brightness normalized to `[0, 1]`.
pub fn tone_at(
    scene: &Scene,
    face_drawable: &raydeon::DrawableShape,
    point: WPoint3,
    normal: WVec3,
) -> f64 {
    let hit = HitData::new(point + normal * SURFACE_LIFT, 1.0, normal);
    let hit_shape = HitShape::new(hit, face_drawable);
    (scene.illumination_for_hit(hit_shape) / TONE_WHITE).clamp(0.0, 1.0)
}

/// Generates parallel lines across `face` at `angle` radians (measured in
/// face coordinates), spaced `spacing` apart, clipped to the outline minus
/// holes. Returns world-space segments.
pub fn face_hatch_lines(
    face: &Face,
    angle: f64,
    spacing: f64,
) -> Vec<LineSegment3D<'static, WorldSpace>> {
    let dir = euclid::Vector2D::<f64, crate::scene::FaceSpace>::new(angle.cos(), angle.sin());
    let perp = euclid::Vector2D::<f64, crate::scene::FaceSpace>::new(-dir.y, dir.x);

    let offsets: Vec<f64> = face
        .outline
        .iter()
        .map(|p| p.to_vector().dot(perp))
        .collect();
    let along: Vec<f64> = face
        .outline
        .iter()
        .map(|p| p.to_vector().dot(dir))
        .collect();
    let (off_min, off_max) = min_max(&offsets);
    let (t_min, t_max) = min_max(&along);

    // Lift lines slightly off the surface so their visibility rays don't
    // immediately re-intersect the face they sit on.
    let lift = face.normal * 0.006;
    let mut segments = Vec::new();
    let mut offset = off_min + spacing / 2.0;
    while offset < off_max {
        let anchor = perp * offset;
        if let Some((t0, t1)) = clip_to_convex(face, anchor, dir, t_min, t_max) {
            for (a, b) in subtract_holes(face, anchor, dir, (t0, t1)) {
                if b - a > 1.0e-6 {
                    let p1 = face.to_world((anchor + dir * a).to_point()) + lift;
                    let p2 = face.to_world((anchor + dir * b).to_point()) + lift;
                    segments.push(LineSegment3D::new_segment(p1, p2));
                }
            }
        }
        offset += spacing;
    }
    segments
}

/// Keeps only the portions of `segment` whose illumination sample passes
/// `keep`, rejoining contiguous runs. `keep` receives the sample's tone and
/// a stable index usable for deterministic jitter.
pub fn filter_by_tone(
    scene: &Scene,
    face_drawable: &raydeon::DrawableShape,
    normal: WVec3,
    segment: &LineSegment3D<'static, WorldSpace>,
    keep: impl Fn(f64, u64) -> bool,
) -> Vec<LineSegment3D<'static, WorldSpace>> {
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
            let tone = tone_at(scene, face_drawable, midpoint, normal);
            let jitter_seed = point_seed(midpoint);
            (!keep(tone, jitter_seed)).then_some(ndx)
        })
        .collect();
    removals
        .into_iter()
        .for_each(|ndx| sliced.remove_subsegment(ndx));
    sliced.join_slices_with_forgiveness(1)
}

/// Hatches every face of the scene with `angle`/`spacing`, filtered by `keep`.
pub fn hatch_faces(
    test: &TestScene,
    angle_for_face: impl Fn(&Face) -> f64,
    spacing: f64,
    keep: impl Fn(f64, u64) -> bool + Copy,
) -> Vec<LineSegment3D<'static, WorldSpace>> {
    test.faces
        .iter()
        .flat_map(|face| {
            face_hatch_lines(face, angle_for_face(face), spacing)
                .iter()
                .flat_map(|line| {
                    filter_by_tone(&test.scene, &face.drawable, face.normal, line, keep)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Latitude-style contour rings over a ball, filtered by `keep`. Rings are
/// perpendicular to `axis` and spaced `spacing` apart along the surface.
pub fn ball_rings(
    scene: &Scene,
    ball: &Ball,
    axis: WVec3,
    spacing: f64,
    keep: impl Fn(f64, u64) -> bool,
) -> Vec<LineSegment3D<'static, WorldSpace>> {
    let axis = axis.normalize();
    let seed = if axis.x.abs() < 0.9 {
        WVec3::new(1.0, 0.0, 0.0)
    } else {
        WVec3::new(0.0, 1.0, 0.0)
    };
    let u = axis.cross(seed).normalize();
    let v = axis.cross(u);
    // Lift rings off the surface so they survive their own occlusion check.
    let radius = ball.radius + 0.006;

    let mut segments = Vec::new();
    let steps = (std::f64::consts::PI * radius / spacing).floor() as usize;
    for step in 1..steps {
        let polar = step as f64 / steps as f64 * std::f64::consts::PI;
        let ring_radius = radius * polar.sin();
        let ring_center = ball.center + axis * (radius * polar.cos());
        let chords = ((ring_radius * std::f64::consts::TAU) / SAMPLE_LEN).ceil() as usize;
        if chords < 8 {
            continue;
        }
        let at = |ndx: usize| {
            let angle = (ndx % chords) as f64 / chords as f64 * std::f64::consts::TAU;
            ring_center + u * (ring_radius * angle.cos()) + v * (ring_radius * angle.sin())
        };
        for ndx in 0..chords {
            let p1 = at(ndx);
            let p2 = at(ndx + 1);
            let midpoint = p1 + (p2 - p1) / 2.0;
            let normal = (midpoint - ball.center).normalize();
            let tone = tone_at(scene, &ball.drawable, midpoint, normal);
            if keep(tone, point_seed(midpoint)) {
                segments.push(LineSegment3D::new_segment(p1, p2));
            }
        }
    }
    segments
}

/// Deterministic pseudo-random value in `[0, 1]` derived from a seed.
pub fn jitter01(seed: u64) -> f64 {
    // splitmix64 finalizer: uniform enough for stochastic hatching.
    let mut z = seed.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z = z ^ (z >> 31);
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Stable seed for a world-space sample point, quantized to ~5mm.
fn point_seed(p: WPoint3) -> u64 {
    let q = |value: f64| (value * 200.0).round() as i64 as u64;
    q(p.x)
        .wrapping_mul(0x9e3779b97f4a7c15)
        .wrapping_add(q(p.y).wrapping_mul(0x85ebca6b))
        .wrapping_add(q(p.z).wrapping_mul(0xc2b2ae35))
}

fn min_max(values: &[f64]) -> (f64, f64) {
    values
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)))
}

/// Clips the parametric line `anchor + t * dir` to the face's convex outline,
/// returning the visible `t` range if any.
fn clip_to_convex(
    face: &Face,
    anchor: euclid::Vector2D<f64, crate::scene::FaceSpace>,
    dir: euclid::Vector2D<f64, crate::scene::FaceSpace>,
    t_min: f64,
    t_max: f64,
) -> Option<(f64, f64)> {
    let mut lo = t_min - 1.0;
    let mut hi = t_max + 1.0;
    let n = face.outline.len();
    for ndx in 0..n {
        let a = face.outline[ndx];
        let b = face.outline[(ndx + 1) % n];
        let edge = b - a;
        // Inward normal for a counter-clockwise outline.
        let inward = euclid::Vector2D::<f64, crate::scene::FaceSpace>::new(-edge.y, edge.x);
        let denom = inward.dot(dir);
        let dist = inward.dot(a.to_vector() - anchor);
        if denom.abs() < 1.0e-12 {
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

/// Removes the sub-intervals of `(t0, t1)` where the line passes through a
/// hole in the face.
fn subtract_holes(
    face: &Face,
    anchor: euclid::Vector2D<f64, crate::scene::FaceSpace>,
    dir: euclid::Vector2D<f64, crate::scene::FaceSpace>,
    range: (f64, f64),
) -> Vec<(f64, f64)> {
    let mut cuts: Vec<(f64, f64)> = face
        .holes
        .iter()
        .filter_map(|hole| {
            let mut lo = f64::NEG_INFINITY;
            let mut hi = f64::INFINITY;
            for axis in 0..2 {
                let (origin, direction) = match axis {
                    0 => (anchor.x, dir.x),
                    _ => (anchor.y, dir.y),
                };
                let (b_lo, b_hi) = match axis {
                    0 => (hole.min.x, hole.max.x),
                    _ => (hole.min.y, hole.max.y),
                };
                if direction.abs() < 1.0e-12 {
                    if origin < b_lo || origin > b_hi {
                        return None;
                    }
                    continue;
                }
                let t1 = (b_lo - origin) / direction;
                let t2 = (b_hi - origin) / direction;
                lo = lo.max(t1.min(t2));
                hi = hi.min(t1.max(t2));
            }
            (hi > lo).then_some((lo, hi))
        })
        .collect();
    cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut result = Vec::new();
    let mut cursor = range.0;
    for (cut_lo, cut_hi) in cuts {
        if cut_hi < cursor || cut_lo > range.1 {
            continue;
        }
        if cut_lo > cursor {
            result.push((cursor, cut_lo));
        }
        cursor = cursor.max(cut_hi);
    }
    if cursor < range.1 {
        result.push((cursor, range.1));
    }
    result
}
