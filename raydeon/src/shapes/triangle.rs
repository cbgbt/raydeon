//! Provides basic drawing and collision for triangles.
use bon::Builder;
use std::sync::Arc;

use super::plane::Plane;
use crate::hatch::surface::{offer_planar, FacePoint, PlanarSurface};
use crate::path::LineSegment3D;
use crate::{
    Camera, CollisionGeometry, HatchSurface, HitData, Ray, Shape, WPoint3, WVec3, WorldSpace,
};

#[derive(Debug, Copy, Clone, Builder)]
#[builder(start_fn(name = new))]
#[cfg_attr(test, derive(PartialEq))]
pub struct Triangle {
    #[builder(into)]
    pub v0: WPoint3,
    #[builder(into)]
    pub v1: WPoint3,
    #[builder(into)]
    pub v2: WPoint3,

    #[builder(skip = [v0, v1, v2])]
    pub verts: [WPoint3; 3],

    #[builder(skip = [v1 - v0, v2 - v1, v0 - v2])]
    pub edges: [WVec3; 3],

    #[builder(skip = Plane::new()
        .point(v0)
        .normal((v1 - v0).cross(v2 - v0).normalize())
        .build()
    )]
    pub plane: Plane,
}

impl Shape for Triangle {
    fn collision_geometry(&self) -> Option<Vec<std::sync::Arc<dyn CollisionGeometry>>> {
        Some(vec![Arc::new(*self)])
    }

    fn paths(&self, _cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        let v0 = self.verts[0];
        let v1 = self.verts[1];
        let v2 = self.verts[2];

        let centroid = (v0 + v1.to_vector() + v2.to_vector()) / 3.0;
        let v0 = v0 + (v0 - centroid).normalize() * 0.015;
        let v1 = v1 + (v1 - centroid).normalize() * 0.015;
        let v2 = v2 + (v2 - centroid).normalize() * 0.015;

        LineSegment3D::from_points(vec![(v0, v1), (v1, v2), (v2, v0)])
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        // The first edge sets the frame's x axis; the plane normal completes
        // it, so the surface faces the way the triangle does.
        let along = (self.v1 - self.v0).normalize();
        let up = self.plane.normal.cross(along);
        let corner = |vert: WPoint3| {
            let offset = vert - self.v0;
            FacePoint::new(offset.dot(along), offset.dot(up))
        };
        let outline = self.verts.iter().map(|vert| corner(*vert)).collect();

        offer_planar(
            PlanarSurface::try_new(self.v0, [along, up], outline, vec![]),
            self,
        )
        .into_iter()
        .collect()
    }
}

impl CollisionGeometry for Triangle {
    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        let p_hit = self.plane.hit_by(ray);

        match p_hit {
            Some(hitdata) => {
                let hit_point = hitdata.hit_point;
                let normal = self.plane.normal;

                let mut gtz = true;
                let mut ltz = true;

                for i in 0..3 {
                    let vp = hit_point - self.verts[i].to_vector();
                    let c = self.edges[i].cross(vp.to_vector());
                    let nc = normal.dot(c);
                    gtz = gtz && nc > 0.0;
                    ltz = ltz && nc < 0.0;
                    if !gtz && !ltz {
                        return None;
                    }
                }

                Some(hitdata)
            }
            None => None,
        }
    }

    fn bounding_box(&self) -> Option<crate::AABB3<crate::WorldSpace>> {
        let mut min = WPoint3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = WPoint3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for vert in &self.verts {
            min.x = min.x.min(vert.x);
            min.y = min.y.min(vert.y);
            min.z = min.z.min(vert.z);

            max.x = max.x.max(vert.x);
            max.y = max.y.max(vert.y);
            max.z = max.z.max(vert.z);
        }

        Some(crate::AABB3::new(min, max))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tri_hit_by() {
        let tri1 = Triangle::new()
            .v0((0.0, 0.0, 0.0))
            .v1((2.0, 0.0, 0.0))
            .v2((0.0, 2.0, 0.0))
            .build();

        // hits tri1
        let ray1 = Ray::new(
            WPoint3::new(0.25, 0.25, -2.0),
            WVec3::new(0.0, 0.0, 1.0).normalize(),
        );

        // does not hit tri1
        let ray2 = Ray::new(
            WPoint3::new(0.1, 2.0, -2.0),
            WVec3::new(0.0, 0.0, 1.0).normalize(),
        );

        // does not hit tri1
        let ray3 = Ray::new(
            WPoint3::new(0.0, 0.0, -2.0),
            WVec3::new(0.0, 0.0, 1.0).normalize(),
        );

        // hits tri1
        let ray4 = Ray::new(
            WPoint3::new(0.1, 0.01, -2.0),
            WVec3::new(0.0, 0.00, 1.0).normalize(),
        );

        assert_eq!(
            tri1.hit_by(&ray1),
            Some(HitData::new(
                WPoint3::new(0.25, 0.25, 0.0),
                2.0,
                (0.0, 0.0, -1.0)
            ))
        );

        assert_eq!(tri1.hit_by(&ray2), None);
        assert_eq!(tri1.hit_by(&ray3), None);

        assert_eq!(
            tri1.hit_by(&ray4),
            Some(HitData::new(
                WPoint3::new(0.1, 0.01, 0.0),
                2.0,
                (0.0, 0.0, -1.0)
            ))
        );
    }
}
