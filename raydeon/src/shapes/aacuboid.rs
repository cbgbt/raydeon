//! Provides basic drawing and collision for axis-aligned cuboids.
use core::f64;
use euclid::Vector3D;
use std::sync::Arc;

use crate::path::LineSegment3D;
use crate::{
    Camera, CollisionGeometry, HitData, Observation, PathMeta, Perspective, Ray, Shape, WPoint3,
    WVec3, WorldSpace, AABB3,
};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct AxisAlignedCuboid<P>
where
    P: PathMeta,
{
    pub min: WVec3,
    pub max: WVec3,
    pub meta: P,
}

impl AxisAlignedCuboid<usize> {
    pub fn new(min: impl Into<WVec3>, max: impl Into<WVec3>) -> AxisAlignedCuboid<usize> {
        Self::tagged(min, max, 0)
    }
}

impl<P: PathMeta> AxisAlignedCuboid<P> {
    pub fn tagged(min: impl Into<WVec3>, max: impl Into<WVec3>, meta: P) -> AxisAlignedCuboid<P> {
        let min = min.into();
        let max = max.into();
        AxisAlignedCuboid { min, max, meta }
    }
}

impl From<AABB3<WorldSpace>> for AxisAlignedCuboid<usize> {
    fn from(value: AABB3<WorldSpace>) -> Self {
        Self::new(value.min.to_vector(), value.max.to_vector())
    }
}

impl<P: PathMeta> Shape<WorldSpace, P> for AxisAlignedCuboid<P> {
    fn metadata(&self) -> P {
        self.meta.clone()
    }

    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry<WorldSpace>>>> {
        Some(vec![Arc::new(self.clone())])
    }

    fn paths(&self, _cam: &Camera<Perspective, Observation>) -> Vec<LineSegment3D<WorldSpace, P>> {
        let expand = (self.max - self.min).normalize() * 0.003;
        let pathmin = self.min - expand;
        let pathmax = self.max + expand;

        let (x1, y1, z1) = (pathmin.x, pathmin.y, pathmin.z);
        let (x2, y2, z2) = (pathmax.x, pathmax.y, pathmax.z);

        let p1 = WPoint3::new(x1, y1, z1);
        let p2 = WPoint3::new(x2, y1, z1);
        let p3 = WPoint3::new(x2, y1, z2);
        let p4 = WPoint3::new(x1, y1, z2);

        let p5 = WPoint3::new(x1, y2, z1);
        let p6 = WPoint3::new(x2, y2, z1);
        let p7 = WPoint3::new(x2, y2, z2);
        let p8 = WPoint3::new(x1, y2, z2);

        vec![
            LineSegment3D::tagged(p1, p2, self.meta.clone()),
            LineSegment3D::tagged(p2, p3, self.meta.clone()),
            LineSegment3D::tagged(p3, p4, self.meta.clone()),
            LineSegment3D::tagged(p4, p1, self.meta.clone()),
            LineSegment3D::tagged(p5, p6, self.meta.clone()),
            LineSegment3D::tagged(p6, p7, self.meta.clone()),
            LineSegment3D::tagged(p7, p8, self.meta.clone()),
            LineSegment3D::tagged(p8, p5, self.meta.clone()),
            LineSegment3D::tagged(p1, p5, self.meta.clone()),
            LineSegment3D::tagged(p2, p6, self.meta.clone()),
            LineSegment3D::tagged(p3, p7, self.meta.clone()),
            LineSegment3D::tagged(p4, p8, self.meta.clone()),
        ]
    }
}

impl<P: PathMeta> CollisionGeometry<WorldSpace> for AxisAlignedCuboid<P> {
    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        let dir_inv = Vector3D::new(1.0, 1.0, 1.0).component_div(ray.dir);
        let t1: Vector3D<f64, WorldSpace> =
            (self.min - ray.point.to_vector()).component_mul(dir_inv);
        let t2: Vector3D<f64, WorldSpace> =
            (self.max - ray.point.to_vector()).component_mul(dir_inv);

        let dir_inv = dir_inv.to_array();
        let t1 = t1.to_array();
        let t2 = t2.to_array();
        let mut hit_normal = [0.0; 3];

        let mut tmin = f64::NEG_INFINITY;
        let mut tmax = f64::INFINITY;

        for i in 0..3 {
            let t1i = t1[i];
            let t2i = t2[i];

            // Ensure t1 is the near plane and t2 is the far plane.
            let (t1i, t2i) = if t1i > t2i { (t2i, t1i) } else { (t1i, t2i) };

            // Update tmin and tmax to track the intersection range.
            if t1i > tmin {
                tmin = t1i;
                // Determine the normal direction.
                hit_normal = [0.0; 3];
                hit_normal[i] = if dir_inv[i] < 0.0 { 1.0 } else { -1.0 };
            }
            tmax = f64::min(tmax, t2i);

            if tmin > tmax {
                return None;
            }
        }

        if tmin < 0.0 {
            return None;
        }

        let hit_point = ray.point + ray.dir * tmin;
        Some(HitData::new(hit_point, tmin, hit_normal))
    }

    fn bounding_box(&self) -> Option<crate::AABB3<crate::WorldSpace>> {
        Some(crate::AABB3::new(self.min.to_point(), self.max.to_point()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rectp_hit_by() {
        let prism1 = AxisAlignedCuboid::new(WVec3::new(0.0, 0.0, 0.0), WVec3::new(1.0, 1.0, 1.0));

        let ray1 = Ray::new(WPoint3::new(-1.0, 0.5, 0.5), WVec3::new(1.0, 0.0, 0.0));

        let ray2 = Ray::new(
            WPoint3::new(-5.0, 10.0, -6.0),
            (WVec3::new(1.0, 0.0, 1.0) - WVec3::new(-5.0, 10.0, -6.0)).normalize(),
        );

        assert_eq!(
            prism1.hit_by(&ray1),
            Some(HitData::new(
                WPoint3::new(0.0, 0.5, 0.5),
                1.0,
                (-1.0, 0.0, 0.0)
            ))
        );

        assert_eq!(
            prism1.hit_by(&ray2),
            Some(HitData::new(
                WPoint3::new(0.39999999999999947, 1.0, 0.29999999999999893),
                12.241323457861899,
                (0.0, 1.0, 0.0)
            ))
        );
    }
}
