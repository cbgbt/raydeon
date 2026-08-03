//! The hatching strategies under comparison.
//!
//! Every strategy renders the same test scene and returns screen-space
//! outline and hatch stroke sets, so results differ only in how shading
//! strokes are conceived.
use crate::scene;
use raydeon::{CameraSpace, HatchSpacing, HatchStyle, Point2, Rendering, StrokeKind};

/// A single screen-space pen stroke.
pub type Stroke = (Point2<CameraSpace>, Point2<CameraSpace>);

/// Screen-space strokes ready for SVG output, split by pen role.
pub struct StrategyRender {
    pub outline: Vec<Stroke>,
    pub hatch: Vec<Stroke>,
}

/// Distance between hatch lines, in world units.
const HATCH_SPACING: f64 = 0.22;

/// The seed the scattered strategies are drawn with.
const SEED: u64 = 42;

/// A strategy paired with the output name of its render.
pub type NamedStrategy = (&'static str, fn() -> StrategyRender);

pub fn all() -> Vec<NamedStrategy> {
    vec![
        ("a_screen_space", screen_space_baseline),
        ("b_stochastic", stochastic),
        ("c_crosshatch_tonal", crosshatch_tonal),
        ("d_light_flow", light_flow),
    ]
}

/// Baseline: raydeon's screen-space hatching, which shades the image rather
/// than the surfaces.
fn screen_space_baseline() -> StrategyRender {
    let test = scene::build(None);
    let scene_camera = test
        .scene
        .attach_camera(test.camera.clone())
        .with_seed(SEED);
    split(&scene_camera.render_with_screen_hatching())
}

/// One direction, thinning out towards the light.
fn stochastic() -> StrategyRender {
    world_hatched(HatchStyle::stochastic(spacing()))
}

/// Discrete tonal bands: darker regions accumulate additional cross-hatch
/// directions at tighter spacing, like an engraving.
fn crosshatch_tonal() -> StrategyRender {
    world_hatched(HatchStyle::tonal_crosshatch(spacing()))
}

/// Strokes which flow along the light across each surface, crossed only in
/// deep shadow.
fn light_flow() -> StrategyRender {
    world_hatched(HatchStyle::light_flow(scene::light_position(), spacing()))
}

fn world_hatched(style: HatchStyle) -> StrategyRender {
    let test = scene::build(Some(style));
    let scene_camera = test
        .scene
        .attach_camera(test.camera.clone())
        .with_seed(SEED);
    split(&scene_camera.render())
}

fn spacing() -> HatchSpacing {
    HatchSpacing::try_new(HATCH_SPACING).expect("the lab's hatch spacing is positive")
}

/// Splits a rendering into the two pens the lab plots with.
fn split(rendering: &Rendering) -> StrategyRender {
    let mut outline = Vec::new();
    let mut hatch = Vec::new();
    for stroke in rendering.strokes() {
        let points = (stroke.p1, stroke.p2);
        match stroke.kind {
            StrokeKind::Outline => outline.push(points),
            StrokeKind::Hatch => hatch.push(points),
        }
    }
    StrategyRender { outline, hatch }
}
