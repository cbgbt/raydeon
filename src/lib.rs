mod rectprism;
pub mod path;
mod plane;
pub mod ray;
pub mod scene;
pub mod shapes;
mod sphere;
mod triangle;

use euclid::*;

pub use path::Paths;
pub use ray::{HitData, Ray};

pub use scene::{Camera, Scene};

#[cfg(test)]
pub(crate) static EPSILON: f64 = 0.004;

#[derive(Debug)]
pub struct WorldSpace;
#[derive(Debug)]
pub struct CameraSpace;

#[derive(Debug)]
pub struct CanvasSpace;

pub type WVec3 = Vector3D<f64, WorldSpace>;
pub type WPoint3 = Point3D<f64, WorldSpace>;

pub type CVec3 = Vector3D<f64, CameraSpace>;
pub type CPoint3 = Point3D<f64, CameraSpace>;

pub type CWTransform = Transform3D<f64, CameraSpace, WorldSpace>;
pub type WCTransform = Transform3D<f64, WorldSpace, CameraSpace>;
pub type WWTransform = Transform3D<f64, WorldSpace, WorldSpace>;
pub type CCTransform = Transform3D<f64, CameraSpace, CameraSpace>;

pub trait Shape {
    fn hit_by(&self, ray: &Ray) -> Option<HitData>;
    fn paths(&self) -> Paths<WorldSpace>;
}
