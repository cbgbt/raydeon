use crate::path::LineSegment;
use crate::{HitData, Ray, Shape, WPoint3, WVec3, WorldSpace};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
/// An infinite plane in 3D space.
pub struct Plane {
    /// An arbitrary point in space which exists on the plane.
    pub point: WPoint3,
    /// A normal vector to the plane.
    pub normal: WVec3,
}

impl Plane {
    pub fn new(point: WPoint3, normal: WVec3) -> Plane {
        Plane { point, normal }
    }
}

impl Shape<WorldSpace> for Plane {
    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        let rdn = ray.dir.dot(self.normal);
        if rdn == 0.0 {
            return None;
        }

        let t = (self.point - ray.point).dot(self.normal) / rdn;

        if t < 0.0 {
            return None;
        }

        let hit_point = ray.point + (ray.dir.normalize() * t);
        Some(HitData::new(hit_point, t))
    }

    fn paths(&self) -> Vec<LineSegment<WorldSpace>> {
        unimplemented!()
    }

    fn bounding_box(&self) -> Option<crate::AABB<crate::WorldSpace>> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hit_by() {
        let plane1 = Plane::new(WPoint3::new(1.0, 0.0, 0.0), WVec3::new(-1.0, 0.0, 0.0));

        assert_eq!(
            plane1.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(WPoint3::new(1.0, 0.0, 0.0), 1.0,))
        );

        assert_eq!(
            plane1.hit_by(&Ray::normalize_new(
                WPoint3::new(0.0, 1.0, 0.0),
                WVec3::new(1.0, -1.0, 0.0)
            )),
            Some(HitData::new(WPoint3::new(1.0, 0.0, 0.0), f64::sqrt(2.0),))
        );

        assert_eq!(
            plane1.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(-1.0, 0.0, 0.0)
            )),
            None
        );

        assert_eq!(
            plane1.hit_by(&Ray::new(
                WPoint3::new(1.1, 0.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            None
        );

        assert_eq!(
            plane1.hit_by(&Ray::normalize_new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(-1.0, 1.0, 0.0)
            )),
            None
        );
    }
}
