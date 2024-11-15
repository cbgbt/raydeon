use crate::{HitData, Paths, Ray, Shape, WPoint3, WVec3};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
/// A sphere at an arbitrary location in 3d space.
pub struct Sphere {
    /// The location of the center of the sphere.
    center: WPoint3,
    /// The radius of the sphere.
    radius: f64,
    /// Precomputed radius squared.
    radius2: f64,
}

impl Sphere {
    pub fn new(center: WPoint3, radius: f64) -> Sphere {
        let radius2 = radius * radius;
        Sphere {
            center,
            radius,
            radius2,
        }
    }
}

impl Shape for Sphere {
    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        let l_vec = self.center - ray.point;
        let t_ca = l_vec.dot(ray.dir);
        let d2 = l_vec.dot(l_vec) - t_ca.powi(2);

        if d2 >= self.radius2 {
            return None;
        }

        let t_hc = (self.radius2 - d2).sqrt();

        let t_0 = t_ca - t_hc;
        let t_1 = t_ca + t_hc;

        if t_0 < 0.0 && t_1 < 0.0 {
            return None;
        }

        let t = if t_0 < 0.0 { t_1 } else { t_0 };

        let hit_point = ray.point + (ray.dir.normalize() * t);
        Some(HitData::new(hit_point, t))
    }

    fn paths(&self) -> Paths<crate::WorldSpace> {
        unimplemented!()
    }

    fn bounding_box(&self) -> Option<crate::AABB<crate::WorldSpace>> {
        let min = self.center - WVec3::splat(self.radius);
        let max = self.center + WVec3::splat(self.radius);
        Some(crate::AABB::new(min, max))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{WPoint3, WVec3};

    #[test]
    fn test_hit_by() {
        let sphere1 = Sphere::new(WPoint3::new(1.0, 0.0, 0.0), 0.5);

        assert_eq!(
            sphere1.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(WPoint3::new(0.5, 0.0, 0.0), 0.5,))
        );

        assert_eq!(
            sphere1.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(-1.0, 0.0, 0.0)
            )),
            None
        );

        assert_eq!(
            sphere1.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.5, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            None
        );

        let sphere2 = Sphere::new(WPoint3::new(1.0, 1.0, 0.0), 0.5);

        assert_eq!(
            sphere2.hit_by(&Ray::new(
                WPoint3::new(0.0, 1.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(WPoint3::new(0.5, 1.0, 0.0), 0.5,))
        );

        let sphere3 = Sphere::new(WPoint3::new(0.0, 0.0, 0.0), 1.0);

        assert_eq!(
            sphere3.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(WPoint3::new(1.0, 0.0, 0.0), 1.0,))
        );

        assert_eq!(
            sphere3.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(-1.0, 0.0, 0.0)
            )),
            Some(HitData::new(WPoint3::new(-1.0, 0.0, 0.0), 1.0,))
        );
    }
}
