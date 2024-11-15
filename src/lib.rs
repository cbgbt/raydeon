pub(crate) mod bvh;
pub mod path;
pub mod ray;
pub mod scene;
pub mod shapes;

use euclid::*;

use path::LineSegment;
pub use ray::{HitData, Ray};

pub use scene::{Camera, Scene};

#[cfg(test)]
pub(crate) static EPSILON: f64 = 0.004;

#[derive(Debug, Copy, Clone)]
pub struct WorldSpace;
#[derive(Debug, Copy, Clone)]
pub struct CameraSpace;

#[derive(Debug, Copy, Clone)]
pub struct CanvasSpace;

pub type WVec3 = Vector3D<f64, WorldSpace>;
pub type WPoint3 = Point3D<f64, WorldSpace>;
pub type AABB<Space> = euclid::Box3D<f64, Space>;

pub type CVec3 = Vector3D<f64, CameraSpace>;
pub type CPoint3 = Point3D<f64, CameraSpace>;

pub type CWTransform = Transform3D<f64, CameraSpace, WorldSpace>;
pub type WCTransform = Transform3D<f64, WorldSpace, CameraSpace>;
pub type WWTransform = Transform3D<f64, WorldSpace, WorldSpace>;
pub type CCTransform = Transform3D<f64, CameraSpace, CameraSpace>;

pub trait Shape<Space>: Send + Sync + std::fmt::Debug
where
    Space: Sized + Send + Sync + std::fmt::Debug + Copy + Clone,
{
    fn hit_by(&self, ray: &Ray) -> Option<HitData>;
    fn paths(&self) -> Vec<LineSegment<Space>>;
    fn bounding_box(&self) -> Option<AABB<Space>>;
}
