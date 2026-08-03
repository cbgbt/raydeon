//! Pins that a render is reproducible down to the order of its strokes: a
//! plotter replays the sequence as given, so a reordered render is a different
//! drawing even when it draws the same lines.

use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{Camera, DrawableShape, Material, PenId, Scene, WPoint3, WVec3};
use std::sync::Arc;

/// Cubes which occlude one another, so the render exercises slicing, clipping
/// and joining rather than whole unbroken edges.
fn occluding_scene() -> Scene {
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
                .material(Material::new().diffuse(3.0).pen(PenId::new(ndx)).build())
                .build()
        })
        .collect::<Vec<_>>();

    Scene::new().geometry(geometry).construct()
}

fn camera() -> Camera {
    Camera::configure()
        .observation(Camera::look_at(
            WPoint3::new(8.0, 6.0, 4.0),
            WVec3::new(0.0, 0.0, 0.0),
            WVec3::new(0.0, 0.0, 1.0),
        ))
        .perspective(Camera::perspective(50.0, 1024, 1024, 0.1, 20.0))
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
