//! The hatching strategies under comparison.
//!
//! Every strategy renders the same test scene and returns screen-space
//! outline and hatch stroke sets, so results differ only in how shading
//! strokes are conceived.
use crate::hatch;
use crate::scene::{Face, TestScene};
use raydeon::{CameraSpace, Point2, SegmentKind, WVec3};

/// A single screen-space pen stroke.
pub type Stroke = (Point2<CameraSpace>, Point2<CameraSpace>);

/// Screen-space strokes ready for SVG output, split by pen role.
pub struct StrategyRender {
    pub outline: Vec<Stroke>,
    pub hatch: Vec<Stroke>,
}

const HATCH_SPACING: f64 = 0.22;
const TIGHT_SPACING: f64 = 0.15;

/// A strategy paired with the output name of its render.
pub type NamedStrategy = (&'static str, fn(&TestScene) -> StrategyRender);

pub fn all() -> Vec<NamedStrategy> {
    vec![
        ("a_screen_space", screen_space_baseline),
        ("b_world_uniform", world_uniform),
        ("c_crosshatch_tonal", crosshatch_tonal),
        ("d_light_flow", light_flow),
    ]
}

/// Baseline: raydeon's current screen-space hatching, seeded for determinism.
fn screen_space_baseline(test: &TestScene) -> StrategyRender {
    let scene_camera = test.scene.attach_camera(test.camera.clone()).with_seed(42);
    let mut outline = Vec::new();
    let mut hatch = Vec::new();
    for segment in scene_camera.render_with_lighting() {
        let points = (segment.p1, segment.p2);
        match segment.kind {
            SegmentKind::Path => outline.push(points),
            SegmentKind::ScreenSpaceHatch(_) => hatch.push(points),
        }
    }
    StrategyRender { outline, hatch }
}

/// World-space hatching at a fixed 45 degree angle per face, with stochastic
/// keep probability driven by illumination.
fn world_uniform(test: &TestScene) -> StrategyRender {
    let keep = |tone: f64, seed: u64| tone < hatch::jitter01(seed) * 0.9;

    let mut lines = hatch::hatch_faces(test, |_| std::f64::consts::FRAC_PI_4, HATCH_SPACING, keep);
    for ball in &test.balls {
        lines.extend(hatch::ball_rings(
            &test.scene,
            ball,
            WVec3::new(0.0, 0.0, 1.0),
            HATCH_SPACING,
            keep,
        ));
    }
    project(test, lines)
}

/// Discrete tonal bands: darker regions accumulate additional cross-hatch
/// directions at tighter spacing, like an engraving.
fn crosshatch_tonal(test: &TestScene) -> StrategyRender {
    let passes: [(f64, f64, f64); 4] = [
        // (angle radians, spacing, tone threshold)
        (std::f64::consts::FRAC_PI_4, HATCH_SPACING, 0.78),
        (3.0 * std::f64::consts::FRAC_PI_4, HATCH_SPACING, 0.55),
        (0.0, TIGHT_SPACING, 0.33),
        (std::f64::consts::FRAC_PI_2, TIGHT_SPACING, 0.16),
    ];

    let mut lines = Vec::new();
    for (angle, spacing, threshold) in passes {
        lines.extend(hatch::hatch_faces(
            test,
            move |_| angle,
            spacing,
            move |tone, _| tone < threshold,
        ));
    }

    let ring_axes = [
        (WVec3::new(0.0, 0.0, 1.0), HATCH_SPACING, 0.78),
        (WVec3::new(1.0, 0.0, 0.0), HATCH_SPACING, 0.55),
        (WVec3::new(0.0, 1.0, 0.0), TIGHT_SPACING, 0.33),
    ];
    for ball in &test.balls {
        for (axis, spacing, threshold) in ring_axes {
            lines.extend(hatch::ball_rings(
                &test.scene,
                ball,
                axis,
                spacing,
                |tone, _| tone < threshold,
            ));
        }
    }
    project(test, lines)
}

/// Hatch strokes flow along each face's projection of the light direction,
/// with a perpendicular cross pass appearing only in deep shadow. Sphere
/// rings wrap around the axis pointing at the light.
fn light_flow(test: &TestScene) -> StrategyRender {
    let flow_keep = |tone: f64, seed: u64| tone < hatch::jitter01(seed) * 0.95;
    let flow_angle = |face: &Face| light_flow_angle(test, face);

    let mut lines = hatch::hatch_faces(test, flow_angle, HATCH_SPACING, flow_keep);
    lines.extend(hatch::hatch_faces(
        test,
        |face| light_flow_angle(test, face) + std::f64::consts::FRAC_PI_2,
        TIGHT_SPACING,
        |tone, _| tone < 0.2,
    ));

    for ball in &test.balls {
        let axis = ball.center - test.light_position;
        lines.extend(hatch::ball_rings(
            &test.scene,
            ball,
            axis,
            HATCH_SPACING,
            flow_keep,
        ));
    }
    project(test, lines)
}

/// The in-plane angle of the light direction projected onto the face.
fn light_flow_angle(test: &TestScene, face: &Face) -> f64 {
    let centroid_2d = face.outline.iter().fold(
        euclid::Vector2D::zero(),
        |acc: euclid::Vector2D<f64, _>, p| acc + p.to_vector(),
    ) / face.outline.len() as f64;
    let centroid = face.to_world(centroid_2d.to_point());
    let to_light = (test.light_position - centroid).normalize();
    let in_plane = to_light - face.normal * to_light.dot(face.normal);
    if in_plane.length() < 1.0e-6 {
        return std::f64::consts::FRAC_PI_4;
    }
    in_plane
        .dot(face.basis[1])
        .atan2(in_plane.dot(face.basis[0]))
}

/// Clips hatch lines against the scene and pairs them with outline geometry.
fn project(
    test: &TestScene,
    lines: Vec<raydeon::LineSegment3D<'static, raydeon::WorldSpace>>,
) -> StrategyRender {
    let scene_camera = test.scene.attach_camera(test.camera.clone());
    let outline = scene_camera
        .render()
        .into_iter()
        .map(|segment| (segment.p1, segment.p2))
        .collect();
    let hatch = scene_camera
        .clip_and_project(&lines)
        .into_iter()
        .map(|segment| (segment.p1, segment.p2))
        .collect();
    StrategyRender { outline, hatch }
}
