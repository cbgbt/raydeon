use crate::{WPoint3, WVec3};

#[cfg(test)]
use euclid::approxeq::ApproxEq as EuclidApproxEq;
#[cfg(test)]
use float_cmp::{approx_eq, ApproxEq};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
/// A ray: a point in space and a direction towards which the ray continues infintely.
pub struct Ray {
    /// The point in space from which the ray originates.
    pub point: WPoint3,
    /// The direction towards which the ray extends. This vector *must* be normalized.
    pub dir: WVec3,
}

impl Ray {
    pub fn new(point: WPoint3, dir: WVec3) -> Ray {
        #[cfg(test)]
        assert!(approx_eq!(f64, dir.length(), 1.0, epsilon = 0.002));
        Ray { point, dir }
    }

    pub fn normalize_new(point: WPoint3, dir: WVec3) -> Ray {
        Ray::new(point, dir.normalize())
    }
}

#[derive(Debug, Copy, Clone)]
/// A structure describing the shape and location that a ray has struck.
pub struct HitData {
    /// The point in space at which the object was hit.
    pub hit_point: WPoint3,
    /// The distance that a ray travelled to hit this shape.
    pub dist_to: f64,
}

impl HitData {
    pub fn new(hit_point: WPoint3, dist_to: f64) -> HitData {
        HitData {
            hit_point,
            dist_to,
        }
    }
}

#[cfg(test)]
use crate::EPSILON;

#[cfg(test)]
impl PartialEq for HitData {
    fn eq(&self, other: &Self) -> bool {
        approx_eq!(&HitData, self, other, epsilon = EPSILON)
    }
}

#[cfg(test)]
impl ApproxEq for &HitData {
    type Margin = float_cmp::F64Margin;

    fn approx_eq<M: Into<Self::Margin>>(self, other: Self, margin: M) -> bool {
        let margin = margin.into();
        self.hit_point.approx_eq(&other.hit_point)
            && self.dist_to.approx_eq(other.dist_to, margin)
    }
}
