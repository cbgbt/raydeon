//! Whole spheres, hatched with contour rings rather than straight lines.

use crate::WPoint3;
use snafu::prelude::*;

/// Why a description of a sphere describes no surface.
#[derive(Debug, Snafu)]
pub enum SphereSurfaceError {
    #[snafu(display("a sphere's radius must be finite and positive, but was {radius}"))]
    NonPositiveRadius { radius: f64 },
}

/// A whole sphere, hatched with contour rings rather than straight lines.
#[derive(Debug, Copy, Clone)]
pub struct SphereSurface {
    center: WPoint3,
    radius: f64,
}

impl SphereSurface {
    pub fn try_new(center: WPoint3, radius: f64) -> Result<Self, SphereSurfaceError> {
        ensure!(
            radius.is_finite() && radius > 0.0,
            NonPositiveRadiusSnafu { radius }
        );
        Ok(Self { center, radius })
    }

    pub fn center(&self) -> WPoint3 {
        self.center
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sphere_needs_a_real_radius() {
        assert!(SphereSurface::try_new(WPoint3::zero(), 0.0).is_err());
        assert!(SphereSurface::try_new(WPoint3::zero(), -1.0).is_err());
        assert!(SphereSurface::try_new(WPoint3::zero(), f64::NAN).is_err());
        assert!(SphereSurface::try_new(WPoint3::zero(), 1.5).is_ok());
    }
}
