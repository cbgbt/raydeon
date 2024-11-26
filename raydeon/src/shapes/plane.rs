//! Provides collision for 3D planes.
use crate::{CollisionGeometry, HitData, Ray, WPoint3, WVec3};
use bon::Builder;

#[derive(Debug, Copy, Clone, Builder)]
#[builder(start_fn(name = new))]
#[cfg_attr(test, derive(PartialEq))]
/// An infinite plane in 3D space.
pub struct Plane {
    /// An arbitrary point in space which exists on the plane.
    #[builder(into)]
    pub point: WPoint3,
    /// A normal vector to the plane.
    #[builder(into)]
    pub normal: WVec3,
}

impl CollisionGeometry for Plane {
    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        let rdn = ray.dir.dot(self.normal);
        if rdn == 0.0 {
            return None;
        }

        let t = (self.point - ray.point).dot(self.normal) / rdn;

        if t < 0.0 {
            return None;
        }

        let hit_norm = if rdn > 0.0 { -self.normal } else { self.normal };
        let hit_point = ray.point + (ray.dir.normalize() * t);
        Some(HitData::new(hit_point, t, hit_norm))
    }

    fn bounding_box(&self) -> Option<crate::AABB3<crate::WorldSpace>> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hit_by() {
        let plane1 = Plane::new()
            .point((1.0, 0.0, 0.0))
            .normal((-1.0, 0.0, 0.0))
            .build();

        assert_eq!(
            plane1.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(
                WPoint3::new(1.0, 0.0, 0.0),
                1.0,
                plane1.normal
            ))
        );

        assert_eq!(
            plane1.hit_by(&Ray::normalize_new(
                WPoint3::new(0.0, 1.0, 0.0),
                WVec3::new(1.0, -1.0, 0.0)
            )),
            Some(HitData::new(
                WPoint3::new(1.0, 0.0, 0.0),
                f64::sqrt(2.0),
                plane1.normal
            ))
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
