//! Provides drawing and collision for spheres.
use crate::hatch::surface::{offer_sphere, SphereSurface};
use crate::path::LineSegment3D;
use crate::{
    Camera, CollisionGeometry, HatchSurface, HitData, Ray, Shape, WPoint3, WVec3, WorldSpace,
};
use bon::Builder;
use std::sync::Arc;

/// How many chords the drawn silhouette is built from.
const CONTOUR_STEPS: usize = 96;

/// The silhouette is drawn on a slightly larger sphere so that it survives
/// its own occlusion check against the sphere it outlines.
const CONTOUR_LIFT: f64 = 0.006;

#[derive(Debug, Copy, Clone, Builder)]
#[builder(start_fn(name = new))]
#[cfg_attr(test, derive(PartialEq))]
/// A sphere at an arbitrary location in 3d space.
pub struct Sphere {
    /// The location of the center of the sphere.
    #[builder(into)]
    pub center: WPoint3,
    /// The radius of the sphere.
    pub radius: f64,
    /// Precomputed radius squared.
    #[builder(skip = radius * radius)]
    radius2: f64,
}

impl Shape for Sphere {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        Some(vec![Arc::new(*self)])
    }

    /// The sphere's silhouette as the camera sees it: the circle where the
    /// surface turns away from the eye, which is nearer the eye and smaller
    /// than a great circle.
    fn paths(&self, cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        let radius = self.radius + CONTOUR_LIFT;
        let to_eye = cam.observation.eye() - self.center;
        let distance = to_eye.length();
        if distance <= radius {
            // The eye is inside the sphere; there is no silhouette to draw.
            return Vec::new();
        }

        let look = to_eye / distance;
        let contour_center = self.center + look * (radius * radius / distance);
        let contour_radius = radius * (1.0 - (radius / distance).powi(2)).sqrt();
        let (u, v) = contour_frame(look);

        let at = |ndx: usize| {
            let angle = (ndx % CONTOUR_STEPS) as f64 / CONTOUR_STEPS as f64 * std::f64::consts::TAU;
            contour_center + u * (contour_radius * angle.cos()) + v * (contour_radius * angle.sin())
        };
        (0..CONTOUR_STEPS)
            .map(|ndx| LineSegment3D::new_segment(at(ndx), at(ndx + 1)))
            .collect()
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        offer_sphere(SphereSurface::try_new(self.center, self.radius), self)
            .into_iter()
            .collect()
    }
}

/// A pair of unit vectors spanning the plane perpendicular to `axis`.
fn contour_frame(axis: WVec3) -> (WVec3, WVec3) {
    let seed = if axis.x.abs() < 0.9 {
        WVec3::new(1.0, 0.0, 0.0)
    } else {
        WVec3::new(0.0, 1.0, 0.0)
    };
    let u = axis.cross(seed).normalize();
    (u, axis.cross(u))
}

impl CollisionGeometry for Sphere {
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
        let hit_normal = (hit_point - self.center).normalize();
        Some(HitData::new(hit_point, t, hit_normal))
    }

    fn bounding_box(&self) -> Option<crate::AABB3<crate::WorldSpace>> {
        let min = self.center - WVec3::splat(self.radius);
        let max = self.center + WVec3::splat(self.radius);
        Some(crate::AABB3::new(min, max))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{WPoint3, WVec3};

    #[test]
    fn test_hit_by() {
        let sphere1 = Sphere::new().center((1.0, 0.0, 0.0)).radius(0.5).build();

        assert_eq!(
            sphere1.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(
                WPoint3::new(0.5, 0.0, 0.0),
                0.5,
                (-1.0, 0.0, 0.0)
            ))
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

        let sphere2 = Sphere::new().center((1.0, 1.0, 0.0)).radius(0.5).build();

        assert_eq!(
            sphere2.hit_by(&Ray::new(
                WPoint3::new(0.0, 1.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(
                WPoint3::new(0.5, 1.0, 0.0),
                0.5,
                (-1.0, 0.0, 0.0)
            ))
        );

        let sphere3 = Sphere::new().center((0.0, 0.0, 0.0)).radius(1.0).build();

        assert_eq!(
            sphere3.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(1.0, 0.0, 0.0)
            )),
            Some(HitData::new(
                WPoint3::new(1.0, 0.0, 0.0),
                1.0,
                (1.0, 0.0, 0.0)
            ))
        );

        assert_eq!(
            sphere3.hit_by(&Ray::new(
                WPoint3::new(0.0, 0.0, 0.0),
                WVec3::new(-1.0, 0.0, 0.0)
            )),
            Some(HitData::new(
                WPoint3::new(-1.0, 0.0, 0.0),
                1.0,
                (-1.0, 0.0, 0.0)
            ))
        );
    }
}
