#[allow(clippy::needless_doctest_main)]
#[doc = include_str!("../../README.md")]
pub(crate) mod bvh;
pub mod camera;
pub mod lights;
pub mod material;
pub mod path;
pub mod ray;
pub mod scene;
pub mod shapes;

use std::sync::Arc;

pub use camera::{Camera, CameraOptions};
pub use lights::Light;
pub use material::Material;
pub use path::LineSegment3D;
pub use ray::{HitData, Ray};
pub use scene::{Scene, SceneGeometry, SceneLighting};

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
pub type WWTransform = Transform3<WorldSpace, WorldSpace>;
pub type CCTransform = Transform3<CameraSpace, CameraSpace>;

pub trait Shape: Send + Sync + std::fmt::Debug {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>>;
    fn metadata(&self) -> Material;
    fn paths(&self, cam: &Camera) -> Vec<LineSegment3D<WorldSpace>>;
}

pub trait CollisionGeometry: Send + Sync + std::fmt::Debug {
    fn hit_by(&self, ray: &Ray) -> Option<HitData>;
    fn bounding_box(&self) -> Option<AABB3<WorldSpace>>;
}
