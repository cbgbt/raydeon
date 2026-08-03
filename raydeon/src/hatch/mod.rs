//! World-space hatching: shading a shape by drawing lines on its own
//! surfaces, so the strokes follow the form and are occluded like the rest of
//! the scene's geometry.
//!
//! A material states a [`HatchStyle`]; a shape offers [`HatchSurface`]s; the
//! engine here decides which lines land where, and how the scene's lighting
//! erodes them.

pub mod contour;
pub mod style;
pub mod surface;

mod dispatch;
pub(crate) mod jitter;
mod lines;

pub use contour::{ContourField, ContourResolution, ContourStyle};
pub use style::{HatchCoverage, HatchSpacing, HatchStyle, TonalPass, ToneThreshold};
pub use surface::{
    FaceBox, FacePoint, FaceSpace, HatchSurface, PlanarSurface, PlanarSurfaceError, ProfilePoint,
    RevolutionSurface, RevolutionSurfaceError, SphereSurface, SphereSurfaceError,
};

use crate::scene::SceneCamera;
use crate::{DrawableShape, LineSegment3D, WorldSpace};

/// The world-space hatch lines `style` calls for on `shape`, already eroded
/// by the scene's lighting but not yet clipped against the scene.
pub(crate) fn hatch_shape(
    cam: &SceneCamera,
    shape: &DrawableShape,
    style: &HatchStyle,
) -> Vec<LineSegment3D<WorldSpace>> {
    shape
        .hatch_surfaces()
        .iter()
        .flat_map(|surface| match surface {
            HatchSurface::Planar(planar) => dispatch::hatch_planar(cam, shape, planar, style),
            HatchSurface::Sphere(sphere) => dispatch::hatch_sphere(cam, shape, sphere, style),
            HatchSurface::Revolution(revolution) => {
                dispatch::hatch_revolution(cam, shape, revolution, style)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lights::PointLight;
    use crate::shapes::Quad;
    use crate::{
        Camera, DrawableShape, Material, PenId, Scene, SceneLighting, Stroke, StrokeKind,
        ToneWhite, WPoint3, WVec3,
    };
    use euclid::Angle;
    use std::sync::Arc;

    /// The pen the floor draws with; the wall draws with the next one.
    const FLOOR_PEN: usize = 0;
    const WALL_PEN: usize = 1;

    fn spacing() -> HatchSpacing {
        HatchSpacing::try_new(0.3).expect("the test spacing is positive")
    }

    /// A floor lit from straight above and a wall which only catches the
    /// light edge on, so the two faces sit in different tonal bands.
    fn two_face_scene(style: HatchStyle) -> Scene {
        let material = |pen: usize| {
            Material::new()
                .diffuse(1.0)
                .pen(PenId::new(pen))
                .hatch(style.clone())
                .build()
        };
        let floor = Quad::new()
            .origin(WPoint3::new(-4.0, -4.0, 0.0))
            .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)])
            .dims([8.0, 6.0])
            .build();
        let wall = Quad::new()
            .origin(WPoint3::new(-4.0, 2.0, 0.0))
            .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 0.0, 1.0)])
            .dims([8.0, 3.0])
            .build();

        Scene::new()
            .geometry(vec![
                DrawableShape::new()
                    .geometry(Arc::new(floor))
                    .material(material(FLOOR_PEN))
                    .build(),
                DrawableShape::new()
                    .geometry(Arc::new(wall))
                    .material(material(WALL_PEN))
                    .build(),
            ])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((0.0, -1.0, 12.0))
                            .intensity(1.0)
                            .build(),
                    )])
                    .with_tone_white(ToneWhite::try_new(1.0).expect("one is a valid tone white")),
            )
            .build()
    }

    fn camera() -> Camera {
        Camera::new()
            .observation(
                Camera::look_at(
                    WPoint3::new(0.0, -14.0, 6.0),
                    WVec3::new(0.0, 0.0, 1.5),
                    WVec3::new(0.0, 0.0, 1.0),
                )
                .expect("the test camera looks at a point in front of it"),
            )
            .perspective(
                Camera::perspective(50.0, 512, 512, 0.1, 40.0)
                    .expect("the test frustum parameters are well formed"),
            )
            .build()
    }

    /// The hatch strokes each pen draws.
    fn hatch_strokes(scene: &Scene, pen: usize) -> Vec<Stroke> {
        scene
            .attach_camera(camera())
            .render()
            .strokes_for_pen(PenId::new(pen))
            .filter(|stroke| stroke.kind == StrokeKind::Hatch)
            .copied()
            .collect()
    }

    #[test]
    fn tonal_hatching_shades_the_darker_face_only() {
        let scene = two_face_scene(HatchStyle::tonal_crosshatch(spacing()));

        assert!(
            hatch_strokes(&scene, FLOOR_PEN).is_empty(),
            "a fully lit face is above every tonal threshold, so it stays blank"
        );
        assert!(
            !hatch_strokes(&scene, WALL_PEN).is_empty(),
            "a face which only grazes the light falls into a tonal band"
        );
    }

    #[test]
    fn each_tonal_band_adds_to_the_one_before_it() {
        let HatchStyle::Tonal { passes } = HatchStyle::tonal_crosshatch(spacing()) else {
            panic!("the tonal preset is a tonal style");
        };
        let first_band = HatchStyle::Tonal {
            passes: passes[..1].to_vec(),
        };

        let banded = hatch_strokes(&two_face_scene(first_band), WALL_PEN).len();
        let all_bands = hatch_strokes(
            &two_face_scene(HatchStyle::tonal_crosshatch(spacing())),
            WALL_PEN,
        )
        .len();

        assert!(
            all_bands > banded && banded > 0,
            "four bands drew {all_bands} strokes where one drew {banded}"
        );
    }

    #[test]
    fn hatching_repeats_stroke_for_stroke() {
        let scene = two_face_scene(HatchStyle::stochastic(spacing()));
        assert_eq!(
            hatch_strokes(&scene, WALL_PEN),
            hatch_strokes(&scene, WALL_PEN),
        );
    }

    #[test]
    fn a_material_without_a_style_draws_no_hatching() {
        let scene = two_face_scene(HatchStyle::tonal_crosshatch(spacing()));
        let unhatched = Scene::new()
            .geometry(vec![DrawableShape::new()
                .geometry(Arc::new(
                    Quad::new()
                        .origin(WPoint3::new(-4.0, 2.0, 0.0))
                        .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 0.0, 1.0)])
                        .dims([8.0, 3.0])
                        .build(),
                ))
                .material(
                    Material::new()
                        .diffuse(1.0)
                        .pen(PenId::new(WALL_PEN))
                        .build(),
                )
                .build()])
            .build();

        assert!(!hatch_strokes(&scene, WALL_PEN).is_empty());
        assert!(hatch_strokes(&unhatched, WALL_PEN).is_empty());
    }

    const LATHE_PEN: usize = 0;

    /// A wheel-thrown cylinder standing on the origin, lit from one side so
    /// part of its curved surface falls into a tonal band.
    fn lathe_scene(style: HatchStyle) -> Scene {
        let lathe = crate::shapes::Lathe::new()
            .base(WPoint3::new(0.0, 0.0, 0.0))
            .profile(vec![
                ProfilePoint {
                    radius: 1.0,
                    height: 0.0,
                },
                ProfilePoint {
                    radius: 1.0,
                    height: 2.0,
                },
            ])
            .build();

        Scene::new()
            .geometry(vec![DrawableShape::new()
                .geometry(Arc::new(lathe))
                .material(
                    Material::new()
                        .diffuse(1.0)
                        .pen(PenId::new(LATHE_PEN))
                        .hatch(style)
                        .build(),
                )
                .build()])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((3.0, 0.0, 1.0))
                            .intensity(1.0)
                            .build(),
                    )])
                    .with_tone_white(ToneWhite::try_new(1.0).expect("one is a valid tone white")),
            )
            .build()
    }

    fn lathe_camera() -> Camera {
        Camera::new()
            .observation(
                Camera::look_at(
                    WPoint3::new(0.0, -6.0, 1.0),
                    WVec3::new(0.0, 0.0, 1.0),
                    WVec3::new(0.0, 0.0, 1.0),
                )
                .expect("the test camera looks at a point in front of it"),
            )
            .perspective(
                Camera::perspective(50.0, 512, 512, 0.1, 20.0)
                    .expect("the test frustum parameters are well formed"),
            )
            .build()
    }

    /// The hatch strokes the lathe's own pen draws, from `lathe_camera`.
    fn lathe_hatch_strokes(scene: &Scene) -> Vec<Stroke> {
        scene
            .attach_camera(lathe_camera())
            .render()
            .strokes_for_pen(PenId::new(LATHE_PEN))
            .filter(|stroke| stroke.kind == StrokeKind::Hatch)
            .copied()
            .collect()
    }

    #[test]
    fn a_tonal_hatched_lathe_emits_hatch_strokes() {
        let scene = lathe_scene(HatchStyle::tonal_crosshatch(spacing()));
        assert!(
            !lathe_hatch_strokes(&scene).is_empty(),
            "rings and meridians should survive their own surface's occlusion check"
        );
    }

    #[test]
    fn tonal_hatching_on_a_revolution_alternates_ring_and_meridian_passes() {
        let ring_spacing = spacing();
        let meridian_spacing = HatchSpacing::try_new(0.45).expect("0.45 is a valid spacing");
        let surface = RevolutionSurface::try_new(
            WPoint3::zero(),
            WVec3::new(0.0, 0.0, 1.0),
            vec![
                ProfilePoint {
                    radius: 1.0,
                    height: 0.0,
                },
                ProfilePoint {
                    radius: 1.0,
                    height: 2.0,
                },
            ],
        )
        .expect("a cylinder is a surface");
        let expected_ring_count = lines::revolution_rings(&surface, ring_spacing).len();
        let expected_meridian_count = lines::revolution_meridians(&surface, meridian_spacing).len();
        assert_ne!(
            expected_ring_count, expected_meridian_count,
            "test fixture sanity check: the two passes must be distinguishable by count"
        );

        // A wide-open threshold keeps every sample, so nothing is chopped
        // away by lighting: the raw hatch output (before the scene clips
        // strokes the shape's own near side occludes) translates the pass
        // counts straight into stroke counts, and the total can only match
        // the ring+meridian split if even passes drew rings and odd passes
        // drew meridians.
        let wide_open = ToneThreshold::try_new(1.0).expect("1.0 is a valid threshold");
        let style = HatchStyle::Tonal {
            passes: vec![
                TonalPass {
                    angle: Angle::degrees(0.0),
                    spacing: ring_spacing,
                    threshold: wide_open,
                },
                TonalPass {
                    angle: Angle::degrees(0.0),
                    spacing: meridian_spacing,
                    threshold: wide_open,
                },
            ],
        };

        let lathe = crate::shapes::Lathe::new()
            .base(WPoint3::zero())
            .profile(surface.profile().to_vec())
            .build();
        let drawable = DrawableShape::new().geometry(Arc::new(lathe)).build();
        let scene = Scene::new()
            .geometry(vec![drawable.clone()])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((3.0, 0.0, 1.0))
                            .intensity(1.0)
                            .build(),
                    )])
                    .with_tone_white(ToneWhite::try_new(1.0).expect("one is a valid tone white")),
            )
            .build();
        let cam = scene.attach_camera(lathe_camera());

        let raw_lines = hatch_shape(&cam, &drawable, &style);
        assert_eq!(
            raw_lines.len(),
            expected_ring_count + expected_meridian_count
        );
    }

    #[test]
    fn light_flow_on_a_revolution_drops_its_cross_pass() {
        let light_flow = |cross_spacing: f64, cross_threshold: f64| HatchStyle::LightFlow {
            source: WPoint3::new(3.0, 0.0, 1.0),
            spacing: spacing(),
            cross_spacing: HatchSpacing::try_new(cross_spacing)
                .expect("test cross spacing is positive"),
            cross_threshold: ToneThreshold::try_new(cross_threshold)
                .expect("test cross threshold is a tone fraction"),
            coverage: crate::HatchCoverage::try_new(0.95).expect("0.95 is a valid coverage"),
        };

        let dropped_cross_a = lathe_hatch_strokes(&lathe_scene(light_flow(0.05, 0.9)));
        let dropped_cross_b = lathe_hatch_strokes(&lathe_scene(light_flow(0.9, 0.01)));

        assert!(!dropped_cross_a.is_empty());
        assert_eq!(
            dropped_cross_a, dropped_cross_b,
            "wildly different cross parameters must draw identically: the cross pass is dropped"
        );
    }
}
