//! The scene model (geometry + lighting) and its rendering pipeline.
//!
//! `Scene`/`SceneGeometry`/`SceneLighting` are the domain: what's in the
//! world and how it's lit. Attaching a `Camera` to a `Scene` produces a
//! `camera::SceneCamera` (`scene::camera`), which owns the render pipeline —
//! turning that world into strokes on the page.

mod camera;

pub use camera::SceneCamera;

use bon::Builder;
use bvh::{BVHTree, Collidable};
use nutype::nutype;
use ray::HitShape;
use std::sync::Arc;

use crate::*;

/// The illumination at which a surface reads as fully white, and above which
/// hatching stops shading it. Tone is `(illumination / tone_white)` clamped
/// to `[0, 1]`, so a scene with brighter lights needs a larger value.
#[nutype(
    validate(finite, greater = 0.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct ToneWhite(f64);

#[derive(Debug, Builder)]
#[builder(start_fn(name = new))]
pub struct Scene {
    #[builder(into)]
    geometry: SceneGeometry,
    #[builder(into, default)]
    lighting: SceneLighting,
}

#[derive(Debug)]
pub struct SceneGeometry {
    geometry: Vec<DrawableShape>,
    bvh: BVHTree,
}

impl SceneGeometry {
    pub fn new() -> SceneGeometry {
        Default::default()
    }

    pub fn with_geometry(mut self, geometry: Vec<DrawableShape>) -> Self {
        let bvh = Self::create_bvh(&geometry);
        self.geometry = geometry;
        self.bvh = bvh;
        self
    }

    fn create_bvh(geometry: &[DrawableShape]) -> BVHTree {
        let collision_geometry: Vec<_> = geometry
            .iter()
            .filter_map(|s| {
                s.collision_geometry().map(|collision_geom| {
                    collision_geom
                        .into_iter()
                        .map(|geom| (s.clone(), geom))
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
            .map(|(s, collision_geom)| Collidable::new(s, collision_geom))
            .collect();
        BVHTree::new(&collision_geometry)
    }
}

impl Default for SceneGeometry {
    fn default() -> Self {
        Self {
            geometry: Vec::new(),
            bvh: BVHTree::new(&[]),
        }
    }
}

impl<S: Shape + 'static> From<Vec<Arc<S>>> for SceneGeometry {
    fn from(geometry: Vec<Arc<S>>) -> Self {
        SceneGeometry::new().with_geometry(
            geometry
                .into_iter()
                .map(|s| DrawableShape::new().geometry(s as Arc<dyn Shape>).build())
                .collect::<Vec<_>>(),
        )
    }
}

impl From<Vec<Arc<dyn Shape>>> for SceneGeometry {
    fn from(shapes: Vec<Arc<dyn Shape>>) -> Self {
        SceneGeometry::new().with_geometry(
            shapes
                .into_iter()
                .map(|shapes| DrawableShape::new().geometry(shapes).build())
                .collect(),
        )
    }
}

impl From<Vec<DrawableShape>> for SceneGeometry {
    fn from(geometry: Vec<DrawableShape>) -> Self {
        SceneGeometry::new().with_geometry(geometry)
    }
}

#[derive(Debug)]
pub struct SceneLighting {
    lights: Vec<Arc<dyn Light>>,
    ambient: f64,
    tone_white: ToneWhite,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            lights: Vec::new(),
            ambient: 0.0,
            tone_white: ToneWhite::try_new(1.0).expect("one is a valid tone normalization"),
        }
    }
}

impl SceneLighting {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_lights(mut self, lights: Vec<Arc<dyn Light>>) -> Self {
        self.lights = lights;
        self
    }

    pub fn with_ambient_lighting(mut self, ambient: f64) -> Self {
        self.ambient = ambient;
        self
    }

    /// Sets the illumination which hatching reads as fully white.
    pub fn with_tone_white(mut self, tone_white: ToneWhite) -> Self {
        self.tone_white = tone_white;
        self
    }

    pub fn tone_white(&self) -> ToneWhite {
        self.tone_white
    }
}

impl<L: Light + 'static> From<Vec<Arc<L>>> for SceneLighting {
    fn from(lights: Vec<Arc<L>>) -> Self {
        let lights = lights
            .into_iter()
            .map(|l| l as Arc<dyn Light>)
            .collect::<Vec<_>>();
        SceneLighting::default().with_lights(lights)
    }
}

impl From<Vec<Arc<dyn Light>>> for SceneLighting {
    fn from(lights: Vec<Arc<dyn Light>>) -> Self {
        SceneLighting::default().with_lights(lights)
    }
}

impl Scene {
    pub fn attach_camera(&self, camera: Camera) -> SceneCamera<'_> {
        SceneCamera::new(camera, self)
    }

    /// Find's the closest intersection point to geometry in the scene, if any
    pub(crate) fn intersects(&self, ray: Ray) -> Option<HitShape<'_>> {
        self.geometry.bvh.intersects(ray)
    }

    /// Computes the total illumination arriving at a surface point from the
    /// scene's lights, including ambient light.
    ///
    /// Callers which already know the surface geometry can construct the
    /// [`HitShape`] directly rather than casting a ray, which is useful for
    /// sampling illumination along surface-space hatch lines.
    ///
    /// `eye` is the viewpoint the illumination is seen from; specular
    /// highlights depend on it.
    pub fn illumination_for_hit<'s>(&'s self, hit: HitShape<'s>, eye: WPoint3) -> f64 {
        self.lighting
            .lights
            .iter()
            .map(|light| light.compute_illumination(self, hit, eye))
            .sum::<f64>()
            + self.lighting.ambient
    }

    /// The perceived brightness at a surface point, normalized to `[0, 1]`
    /// by the scene's `tone_white`: 0 is as dark as hatching shades, 1 is
    /// paper white.
    pub(crate) fn tone_for_hit(&self, hit: HitShape, eye: WPoint3) -> f64 {
        (self.illumination_for_hit(hit, eye) / self.lighting.tone_white.into_inner())
            .clamp(0.0, 1.0)
    }

    /// Returns whether or not the given camera has a clear line of sight to a given point.
    ///
    /// A point is occluded iff the nearest hit along the ray toward the eye
    /// lies strictly BETWEEN the point and the eye; a hit at or beyond the
    /// eye (e.g. geometry behind the camera) never occludes (invariant 21).
    fn visible(&self, from: WPoint3, point: WPoint3) -> bool {
        let v = from - point;
        let r = Ray::new(point, v.normalize());

        match self.intersects(r) {
            Some(hit_shape) => hit_shape.hit_data.dist_to >= v.length() - OCCLUSION_TOL,
            None => true,
        }
    }
}

/// Tolerance on the eye-distance comparison in `Scene::visible`: a hit within
/// this margin of the eye is treated as "at the eye", not an occluder.
const OCCLUSION_TOL: f64 = 1.0e-1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::AxisAlignedCuboid;

    fn cube_scene(center: f64) -> Scene {
        let cuboid = AxisAlignedCuboid::new()
            .min((center - 0.5, -0.5, -0.5))
            .max((center + 0.5, 0.5, 0.5))
            .build();
        Scene::new()
            .geometry(vec![Arc::new(cuboid) as Arc<dyn Shape>])
            .build()
    }

    #[test]
    fn an_occluder_between_the_point_and_the_eye_hides_it() {
        let scene = cube_scene(5.0);
        let point = WPoint3::new(0.0, 0.0, 0.0);
        let eye = WPoint3::new(10.0, 0.0, 0.0);

        assert!(!scene.visible(eye, point));
    }

    #[test]
    fn geometry_behind_the_eye_does_not_occlude() {
        let scene = cube_scene(20.0);
        let point = WPoint3::new(0.0, 0.0, 0.0);
        let eye = WPoint3::new(10.0, 0.0, 0.0);

        assert!(scene.visible(eye, point));
    }
}
