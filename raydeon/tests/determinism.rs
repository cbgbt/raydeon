//! Pins that a render is reproducible down to the order of its strokes: a
//! plotter replays the sequence as given, so a reordered render is a different
//! drawing even when it draws the same lines.

use raydeon::lights::PointLight;
use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{
    Camera, DrawableShape, HatchSpacing, HatchStyle, Material, PenId, Scene, SceneLighting,
    WPoint3, WVec3,
};
use std::sync::Arc;

/// Cubes which occlude one another, so the render exercises slicing, clipping
/// and joining rather than whole unbroken edges.
fn occluding_scene() -> Scene {
    scene_of(None)
}

/// The same cubes, shaded by a lit hatching style, so the render exercises
/// the position-hashed noise as well.
fn hatched_scene() -> Scene {
    let spacing = HatchSpacing::try_new(0.3).expect("the test spacing is positive");
    scene_of(Some(HatchStyle::stochastic(spacing)))
}

fn scene_of(hatch: Option<HatchStyle>) -> Scene {
    let cuboids = [
        ((-1.0, -1.0, -1.0), (1.0, 1.0, 1.0)),
        ((1.8, -1.0, -1.0), (3.8, 1.0, 1.0)),
        ((-1.4, 1.8, -1.0), (0.6, 3.8, 1.0)),
        ((-0.5, -0.5, 1.0), (0.5, 0.5, 2.6)),
    ];

    let geometry = cuboids
        .into_iter()
        .enumerate()
        .map(|(ndx, (min, max))| {
            DrawableShape::new()
                .geometry(Arc::new(AxisAlignedCuboid::new().min(min).max(max).build()))
                .material(
                    Material::new()
                        .diffuse(3.0)
                        .pen(PenId::new(ndx))
                        .maybe_hatch(hatch.clone())
                        .build(),
                )
                .build()
        })
        .collect::<Vec<_>>();

    Scene::new()
        .geometry(geometry)
        .lighting(
            SceneLighting::new()
                .with_lights(vec![Arc::new(
                    PointLight::new()
                        .position((5.5, 12.0, 7.3))
                        .intensity(20.0)
                        .constant_attenuation(0.0)
                        .linear_attenuation(0.09)
                        .quadratic_attenuation(0.23)
                        .build(),
                )])
                .with_ambient_lighting(0.13),
        )
        .build()
}

fn camera() -> Camera {
    Camera::new()
        .observation(
            Camera::look_at(
                WPoint3::new(8.0, 6.0, 4.0),
                WVec3::new(0.0, 0.0, 0.0),
                WVec3::new(0.0, 0.0, 1.0),
            )
            .expect("the test camera looks at a point in front of it"),
        )
        .perspective(
            Camera::perspective(50.0, 1024, 1024, 0.1, 20.0)
                .expect("the test frustum parameters are well formed"),
        )
        .build()
}

#[test]
fn renders_the_same_ordered_stroke_sequence_every_time() {
    let scene = occluding_scene();
    let scene_camera = scene.attach_camera(camera());

    let first = scene_camera.render();
    let second = scene_camera.render();

    assert!(
        !first.strokes().is_empty(),
        "the scene rendered nothing, so this pins nothing"
    );
    assert_eq!(first.strokes(), second.strokes());
}

#[test]
fn renders_the_same_sequence_from_a_freshly_built_scene() {
    let first = occluding_scene().attach_camera(camera()).render();
    let second = occluding_scene().attach_camera(camera()).render();

    assert_eq!(first.strokes(), second.strokes());
}

#[test]
fn renders_the_same_hatching_every_time() {
    let scene = hatched_scene();
    let scene_camera = scene.attach_camera(camera());

    let first = scene_camera.render();
    let second = scene_camera.render();

    assert!(
        first.strokes().len()
            > occluding_scene()
                .attach_camera(camera())
                .render()
                .strokes()
                .len(),
        "the hatched scene drew no more strokes than the unhatched one, so this pins nothing"
    );
    assert_eq!(first.strokes(), second.strokes());
}

#[test]
fn renders_the_same_screen_hatching_every_time() {
    let scene = occluding_scene();
    let scene_camera = scene.attach_camera(camera());

    let first = scene_camera.render_with_screen_hatching();
    let second = scene_camera.render_with_screen_hatching();

    assert!(
        first.strokes().len() > scene_camera.render().strokes().len(),
        "screen hatching added no strokes, so this pins nothing"
    );
    assert_eq!(first.strokes(), second.strokes());
}

#[test]
fn a_different_seed_draws_a_different_scatter() {
    let scene = hatched_scene();
    let camera = camera();

    let unseeded = scene.attach_camera(camera.clone()).render();
    let seeded = scene.attach_camera(camera).with_seed(17).render();

    assert_ne!(unseeded.strokes(), seeded.strokes());
}
