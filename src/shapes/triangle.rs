use super::plane::Plane;
use crate::path::LineSegment;
use crate::{HitData, Ray, Shape, WPoint3, WVec3, WorldSpace};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Triangle {
    pub verts: [WPoint3; 3],
    pub edges: [WVec3; 3],
    pub plane: Plane,
    pub tag: usize,
}

impl Triangle {
    pub fn new(v0: WPoint3, v1: WPoint3, v2: WPoint3) -> Triangle {
        Self::tagged(v0, v1, v2, 0)
    }

    pub fn tagged(v0: WPoint3, v1: WPoint3, v2: WPoint3, tag: usize) -> Triangle {
        let verts = [v0, v1, v2];
        let edges = [v1 - v0, v2 - v1, v0 - v2];
        let normal = (v1 - v0).cross(v2 - v0).normalize();
        let plane = Plane::new(v0, normal);
        Triangle {
            verts,
            edges,
            plane,
            tag,
        }
    }
}

impl Shape<WorldSpace> for Triangle {
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

    fn paths(&self) -> Vec<LineSegment<WorldSpace>> {
        let v0 = self.verts[0];
        let v1 = self.verts[1];
        let v2 = self.verts[2];

        let centroid = (v0 + v1.to_vector() + v2.to_vector()) / 3.0;
        let v0 = v0 + (v0 - centroid).normalize() * 0.015;
        let v1 = v1 + (v1 - centroid).normalize() * 0.015;
        let v2 = v2 + (v2 - centroid).normalize() * 0.015;

        vec![
            LineSegment::tagged(v0, v1, self.tag),
            LineSegment::tagged(v1, v2, self.tag),
            LineSegment::tagged(v2, v0, self.tag),
        ]
    }

    fn bounding_box(&self) -> Option<crate::AABB<crate::WorldSpace>> {
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

        Some(crate::AABB::new(min, max))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tri_hit_by() {
        let tri1 = Triangle::new(
            WPoint3::new(0.0, 0.0, 0.0),
            WPoint3::new(2.0, 0.0, 0.0),
            WPoint3::new(0.0, 2.0, 0.0),
        );

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
            Some(HitData::new(WPoint3::new(0.25, 0.25, 0.0), 2.0))
        );

        assert_eq!(tri1.hit_by(&ray2), None);
        assert_eq!(tri1.hit_by(&ray3), None);

        assert_eq!(
            tri1.hit_by(&ray4),
            Some(HitData::new(WPoint3::new(0.1, 0.01, 0.0), 2.0))
        );
    }
}
