//! The surfaces a shape offers up to be hatched.
//!
//! A hatchable surface is not the shape itself: it is the piece of geometry
//! hatch lines are drawn across, in a frame those lines can be laid out in.
//! Every kind is parsed on the way in, so the engine can lay lines out
//! without re-checking that the frame makes sense.

mod planar;
mod revolution;
mod sphere;

pub use planar::{FaceBox, FacePoint, FaceSpace, PlanarSurface, PlanarSurfaceError};
pub use revolution::{ProfilePoint, RevolutionSurface, RevolutionSurfaceError};
pub use sphere::{SphereSurface, SphereSurfaceError};

/// A piece of a shape's surface which hatch lines can be drawn across.
#[derive(Debug, Clone)]
pub enum HatchSurface {
    Planar(PlanarSurface),
    Sphere(SphereSurface),
    Revolution(RevolutionSurface),
}

/// Offers a parsed planar surface for hatching, or nothing at all.
///
/// A shape reports the surfaces it has; it has no caller to hand a parse
/// failure to. A description which does not describe a surface therefore
/// yields no surface, and says why in the log.
pub(crate) fn offer_planar(
    parsed: Result<PlanarSurface, PlanarSurfaceError>,
    shape: &impl std::fmt::Debug,
) -> Option<HatchSurface> {
    match parsed {
        Ok(surface) => Some(HatchSurface::Planar(surface)),
        Err(error) => {
            tracing::warn!("{shape:?} offers no hatchable surface: {error}");
            None
        }
    }
}

/// Offers a parsed sphere for hatching, or nothing at all.
pub(crate) fn offer_sphere(
    parsed: Result<SphereSurface, SphereSurfaceError>,
    shape: &impl std::fmt::Debug,
) -> Option<HatchSurface> {
    match parsed {
        Ok(surface) => Some(HatchSurface::Sphere(surface)),
        Err(error) => {
            tracing::warn!("{shape:?} offers no hatchable surface: {error}");
            None
        }
    }
}

/// Offers a parsed surface of revolution for hatching, or nothing at all.
pub(crate) fn offer_revolution(
    parsed: Result<RevolutionSurface, RevolutionSurfaceError>,
    shape: &impl std::fmt::Debug,
) -> Option<HatchSurface> {
    match parsed {
        Ok(surface) => Some(HatchSurface::Revolution(surface)),
        Err(error) => {
            tracing::warn!("{shape:?} offers no hatchable surface: {error}");
            None
        }
    }
}
