//! Provides basic drawing and collision for axis-aligned cuboids.
use bon::Builder;
use core::f64;
use euclid::Vector3D;
use std::sync::Arc;

use crate::path::LineSegment3D;
use crate::{
    Camera, CollisionGeometry, HitData, Material, Ray, Shape, WPoint3, WVec3, WorldSpace, AABB3,
};

#[derive(Debug, Copy, Clone, Builder)]
#[builder(start_fn(name = new))]
#[cfg_attr(test, derive(PartialEq))]
pub struct AxisAlignedCuboid {
    #[builder(into)]
    pub min: WVec3,
    #[builder(into)]
    pub max: WVec3,
    pub material: Option<Material>,
}

impl From<AABB3<WorldSpace>> for AxisAlignedCuboid {
    fn from(value: AABB3<WorldSpace>) -> Self {
        Self::new()
            .min(value.min.to_vector())
            .max(value.max.to_vector())
            .build()
    }
}

impl Shape for AxisAlignedCuboid {
    fn metadata(&self) -> Material {
        self.material.unwrap_or_default()
    }

    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        Some(vec![Arc::new(*self)])
    }

    fn paths(&self, _cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
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

        LineSegment3D::from_points(
            vec![
                (p1, p2),
                (p2, p3),
                (p3, p4),
                (p4, p1),
                (p5, p6),
                (p6, p7),
                (p7, p8),
                (p8, p5),
                (p1, p5),
                (p2, p6),
                (p3, p7),
                (p4, p8),
            ],
            self.material,
        )
    }
}

impl CollisionGeometry for AxisAlignedCuboid {
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
        let prism1 = AxisAlignedCuboid::new()
            .min((0.0, 0.0, 0.0))
            .max((1.0, 1.0, 1.0))
            .build();

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
