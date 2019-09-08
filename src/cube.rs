use crate::{HitData, Paths, Ray, Shape, WPoint3, WVec3};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Cube {
    pub min: WVec3,
    pub max: WVec3,
}

impl Cube {
    pub fn new(min: WVec3, max: WVec3) -> Cube {
        Cube { min, max }
    }
}

impl Shape for Cube {
    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        let n = self.min - ray.point.to_vector();
        let n = WVec3::new(n.x / ray.dir.x, n.y / ray.dir.y, n.z / ray.dir.z);

        let f = self.max - ray.point.to_vector();
        let f = WVec3::new(f.x / ray.dir.x, f.y / ray.dir.y, f.z / ray.dir.z);

        let n = n.min(f);
        let f = n.max(f);

        let t0 = f64::max(f64::max(n.x, n.y), n.z);
        let t1 = f64::min(f64::min(f.x, f.y), f.z);


        if t0 < 1.0e-3 && t1 > 1.0e-3 {
            let hitpoint = ray.point + ray.dir * t1;
            Some(HitData::new(hitpoint, (hitpoint - ray.point).length()))
        } else if t0 > 1.0e-3 && t0 < t1 {
            let hitpoint = ray.point + ray.dir * t0;
            Some(HitData::new(hitpoint, (hitpoint - ray.point).length()))
        } else {
            None
        }
    }

    fn paths(&self) -> Paths<crate::WorldSpace> {
        let (x1, y1, z1) = (self.min.x, self.min.y, self.min.z);
        let (x2, y2, z2) = (self.max.x, self.max.y, self.max.z);

        let p1 = WPoint3::new(x1, y1, z1);
        let p2 = WPoint3::new(x2, y1, z1);
        let p3 = WPoint3::new(x2, y1, z2);
        let p4 = WPoint3::new(x1, y1, z2);

        let p5 = WPoint3::new(x1, y2, z1);
        let p6 = WPoint3::new(x2, y2, z1);
        let p7 = WPoint3::new(x2, y2, z2);
        let p8 = WPoint3::new(x1, y2, z2);

        // Make paths lines, not polylines :(
        Paths::new(vec![
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
            (p4, p8)
        ])
    }
}