//! Pins the exact illumination the lighting model produces for a known
//! surface point, so a change to the diffuse or attenuation math shows up as a
//! failing number rather than as a subtly different picture.

use raydeon::lights::PointLight;
use raydeon::ray::{HitData, HitShape};
use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{DrawableShape, Material, Scene, SceneLighting, WPoint3, WVec3};
use std::sync::Arc;

/// Illumination arriving at the top face of the cuboid below, hand-derived
/// from the light's parameters:
///
/// ```text
/// to_light  = (5.5, 12.0, 6.3),  |to_light| = 14.626687936781860
/// diffuse   = material.diffuse * intensity * (to_light.normalize() . normal)
///           = 3.0 * 20.0 * 0.430719519499512 = 25.843171169970756
/// atten     = 1 / (0.0 + 0.09 * 14.626687936781860
///                      + 0.23 * 14.626687936781860^2) = 0.019793121535903
/// illum     = diffuse * atten + ambient = 0.511517027840381 + 0.13
/// ```
///
/// The light's specular intensity is 0, so the specular term contributes
/// nothing and this constant is independent of where the eye sits.
const EXPECTED_ILLUMINATION: f64 = 0.6415170278403806;

#[test]
fn illumination_matches_the_hand_derived_value() {
    let material = Material::new()
        .diffuse(3.0)
        .specular(2.0)
        .shininess(2.0)
        .build();
    let cuboid = DrawableShape::new()
        .geometry(Arc::new(
            AxisAlignedCuboid::new()
                .min((-1.0, -1.0, -1.0))
                .max((1.0, 1.0, 1.0))
                .build(),
        ))
        .material(material)
        .build();

    let scene = Scene::new()
        .geometry(vec![cuboid.clone()])
        .lighting(
            SceneLighting::new()
                .with_lights(vec![Arc::new(
                    PointLight::new()
                        .position((5.5, 12.0, 7.3))
                        .intensity(20.0)
                        .specular_intensity(0.0)
                        .constant_attenuation(0.0)
                        .linear_attenuation(0.09)
                        .quadratic_attenuation(0.23)
                        .build(),
                )])
                .with_ambient_lighting(0.13),
        )
        .build();

    // The centre of the cuboid's top face, which the light sees directly.
    let hit_data = HitData::new(WPoint3::new(0.0, 0.0, 1.0), 5.0, WVec3::new(0.0, 0.0, 1.0));
    // The light's specular intensity is 0, so this eye position cannot
    // influence the result — it only has to be somewhere.
    let eye = WPoint3::new(8.0, 6.0, 4.0);
    let illumination = scene.illumination_for_hit(HitShape::new(hit_data, &cuboid), eye);

    assert!(
        (illumination - EXPECTED_ILLUMINATION).abs() < 1e-9,
        "illumination was {illumination}, expected {EXPECTED_ILLUMINATION}"
    );
}
