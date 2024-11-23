//! Provides basic drawing and collision for axis-aligned cuboids.
use std::sync::Arc;

use collision::Continuous;

use crate::path::LineSegment3D;
use crate::{
    Camera, CollisionGeometry, HitData, Observation, Perspective, Ray, Shape, WPoint3, WVec3,
    WorldSpace, AABB3,
};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct AxisAlignedCuboid {
    pub min: WVec3,
    pub max: WVec3,
    pub tag: usize,
}

impl AxisAlignedCuboid {
    pub fn new(min: WVec3, max: WVec3) -> AxisAlignedCuboid {
        Self::tagged(min, max, 0)
    }

    pub fn tagged(min: WVec3, max: WVec3, tag: usize) -> AxisAlignedCuboid {
        AxisAlignedCuboid { min, max, tag }
    }
}

impl From<AABB3<WorldSpace>> for AxisAlignedCuboid {
    fn from(value: AABB3<WorldSpace>) -> Self {
        Self::new(value.min.to_vector(), value.max.to_vector())
    }
}

impl Shape<WorldSpace> for AxisAlignedCuboid {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry<WorldSpace>>>> {
        Some(vec![Arc::new(*self)])
    }

    fn paths(&self, _cam: &Camera<Perspective, Observation>) -> Vec<LineSegment3D<WorldSpace>> {
        let expand = (self.max - self.min).normalize() * 0.0015;
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
            LineSegment3D::tagged(p1, p2, self.tag),
            LineSegment3D::tagged(p2, p3, self.tag),
            LineSegment3D::tagged(p3, p4, self.tag),
            LineSegment3D::tagged(p4, p1, self.tag),
            LineSegment3D::tagged(p5, p6, self.tag),
            LineSegment3D::tagged(p6, p7, self.tag),
            LineSegment3D::tagged(p7, p8, self.tag),
            LineSegment3D::tagged(p8, p5, self.tag),
            LineSegment3D::tagged(p1, p5, self.tag),
            LineSegment3D::tagged(p2, p6, self.tag),
            LineSegment3D::tagged(p3, p7, self.tag),
            LineSegment3D::tagged(p4, p8, self.tag),
        ]
    }
}

impl CollisionGeometry<WorldSpace> for AxisAlignedCuboid {
    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        let aabb = collision::Aabb3::new(
            cgmath::Point3::new(self.min.x, self.min.y, self.min.z),
            cgmath::Point3::new(self.max.x, self.max.y, self.max.z),
        );
        let r = collision::Ray3::new(
            cgmath::Point3::new(ray.point.x, ray.point.y, ray.point.z),
            cgmath::Vector3::new(ray.dir.x, ray.dir.y, ray.dir.z),
        );

        match r.intersection(&aabb) {
            Some(p) => {
                let wp = WPoint3::new(p.x, p.y, p.z);
                let dist = (wp - ray.point).length();
                Some(HitData::new(wp, dist))
            }
            None => None,
        }
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
            Some(HitData::new(WPoint3::new(0.0, 0.5, 0.5), 1.0))
        );

        assert_eq!(
            prism1.hit_by(&ray2),
            Some(HitData::new(
                WPoint3::new(0.39999999999999947, 1.0, 0.29999999999999893),
                12.241323457861899
            ))
        );
    }
}
