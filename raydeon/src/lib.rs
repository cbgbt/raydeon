use bon::Builder;

#[allow(clippy::needless_doctest_main)]
#[doc = include_str!("../../README.md")]
pub(crate) mod bvh;
pub mod camera;
pub mod hatch;
pub mod lights;
pub mod material;
pub mod path;
pub mod ray;
pub mod scene;
pub mod shapes;
pub mod stroke;

use std::fmt::Debug;
use std::sync::Arc;

pub use camera::{Camera, CameraOptions, LookAtError, Observation, Perspective, PerspectiveError};
pub use hatch::contour::{ContourField, ContourResolution, ContourStyle};
pub use hatch::style::{HatchCoverage, HatchSpacing, HatchStyle, TonalPass, ToneThreshold};
pub use hatch::surface::{
    FaceBox, FacePoint, FaceSpace, HatchSurface, PlanarSurface, PlanarSurfaceError, SphereSurface,
    SphereSurfaceError,
};
pub use lights::Light;
pub use material::Material;
pub use path::LineSegment3D;
pub use ray::{HitData, Ray};
pub use scene::{Scene, SceneGeometry, SceneLighting, ToneWhite};
pub use stroke::{PenId, Rendering, Stroke, StrokeKind};

/// Tolerance for the approximate geometric comparisons in tests.
#[cfg(test)]
pub(crate) static EPSILON: f64 = 0.004;

#[derive(Debug, Copy, Clone)]
pub struct WorldSpace;
#[derive(Debug, Copy, Clone)]
pub struct CameraSpace;

#[derive(Debug, Copy, Clone)]
pub struct CanvasSpace;

pub type Point2<Space> = euclid::Point2D<f64, Space>;
pub type Point3<Space> = euclid::Point3D<f64, Space>;
pub type Vec2<Space> = euclid::Vector2D<f64, Space>;
pub type Vec3<Space> = euclid::Vector3D<f64, Space>;
pub type Transform2<FromSpace, DstSpace> = euclid::Transform2D<f64, FromSpace, DstSpace>;
pub type Transform3<FromSpace, DstSpace> = euclid::Transform3D<f64, FromSpace, DstSpace>;
pub type AABB3<Space> = euclid::Box3D<f64, Space>;

pub type WVec3 = Vec3<WorldSpace>;
pub type WPoint3 = Point3<WorldSpace>;

pub type CVec3 = Vec3<CameraSpace>;
pub type CPoint3 = Point3<CameraSpace>;

pub type CWTransform = Transform3<CameraSpace, WorldSpace>;
pub type WCTransform = Transform3<WorldSpace, CameraSpace>;

pub trait Shape: Send + Sync + std::fmt::Debug {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>>;
    fn paths(&self, cam: &Camera) -> Vec<LineSegment3D<WorldSpace>>;

    /// The surfaces this shape offers up for world-space hatching.
    ///
    /// Returning no surfaces is a complete answer: it says this shape is not
    /// hatchable, and a material's hatch style has nothing to draw on.
    fn hatch_surfaces(&self) -> Vec<HatchSurface>;
}

pub trait CollisionGeometry: Send + Sync + std::fmt::Debug {
    fn hit_by(&self, ray: &Ray) -> Option<HitData>;
    fn bounding_box(&self) -> Option<AABB3<WorldSpace>>;
}

#[derive(Debug, Clone, Builder)]
#[builder(start_fn(name = new))]
pub struct DrawableShape {
    geometry: Arc<dyn Shape>,
    material: Option<Material>,
}

impl DrawableShape {
    pub fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        self.geometry.collision_geometry()
    }

    pub fn paths(&self, cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        self.geometry.paths(cam)
    }

    pub fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        self.geometry.hatch_surfaces()
    }

    pub fn material(&self) -> Option<&Material> {
        self.material.as_ref()
    }

    /// The underlying geometry, exposed in-crate so tests can assert hit
    /// identity via `Arc::ptr_eq` (bvh.rs's brute-force equivalence pin).
    #[cfg(test)]
    pub(crate) fn geometry(&self) -> &Arc<dyn Shape> {
        &self.geometry
    }
}
