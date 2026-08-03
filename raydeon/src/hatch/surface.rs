//! The surfaces a shape offers up to be hatched.
//!
//! A hatchable surface is not the shape itself: it is the piece of geometry
//! hatch lines are drawn across, in a frame those lines can be laid out in.
//! Both kinds are parsed on the way in, so the engine can lay lines out
//! without re-checking that the frame makes sense.

use crate::{Point2, WPoint3, WVec3};
use euclid::Angle;
use snafu::prelude::*;

/// Coordinates within a planar surface's own frame, in world units along the
/// surface's basis vectors.
#[derive(Debug, Copy, Clone)]
pub struct FaceSpace;

pub type FacePoint = Point2<FaceSpace>;
pub type FaceBox = euclid::Box2D<f64, FaceSpace>;

/// Below this length a basis vector, or a polygon's area, is indistinguishable
/// from nothing.
const MIN_EXTENT: f64 = 1.0e-9;

/// How far from perpendicular a basis pair may be and still describe the
/// frame its outline and holes were written in.
const MAX_SKEW: f64 = 1.0e-9;

/// A piece of a shape's surface which hatch lines can be drawn across.
#[derive(Debug, Clone)]
pub enum HatchSurface {
    Planar(PlanarSurface),
    Sphere(SphereSurface),
}

/// Why a description of a planar surface describes no surface.
#[derive(Debug, Snafu)]
pub enum PlanarSurfaceError {
    #[snafu(display(
        "the basis vectors must be non-zero and perpendicular to describe the frame the \
         outline is written in"
    ))]
    DegenerateBasis,
    #[snafu(display("an outline needs at least three corners enclosing some area, but had {corners} corners enclosing {area}"))]
    OutlineTooSmall { corners: usize, area: f64 },
    #[snafu(display("the outline turns back on itself at corner {corner}, and hatch lines can only be clipped to a convex outline"))]
    OutlineNotConvex { corner: usize },
}

/// Why a description of a sphere describes no surface.
#[derive(Debug, Snafu)]
pub enum SphereSurfaceError {
    #[snafu(display("a sphere's radius must be finite and positive, but was {radius}"))]
    NonPositiveRadius { radius: f64 },
}

/// A convex planar region, optionally with rectangular openings which hatch
/// lines skip.
///
/// The fields are private because the frame, the winding and the convexity
/// of the outline are what let the engine lay lines out with plain interval
/// math; [`PlanarSurface::try_new`] is the only way to mint one.
#[derive(Debug, Clone)]
pub struct PlanarSurface {
    origin: WPoint3,
    /// Orthonormal; `basis[0].cross(basis[1])` is the outward normal.
    basis: [WVec3; 2],
    /// Convex, counter-clockwise, in the orthonormal frame.
    outline: Vec<FacePoint>,
    holes: Vec<FaceBox>,
}

impl PlanarSurface {
    /// Parses a planar surface from an origin, a pair of in-plane basis
    /// vectors, and an outline with holes expressed in units of those
    /// vectors.
    ///
    /// The basis is normalized and the outline and holes are re-expressed in
    /// the normalized frame, so callers may describe a face in whatever scale
    /// suits it. A clockwise outline is reversed to counter-clockwise, which
    /// is the winding the outward normal `basis[0].cross(basis[1])` implies.
    pub fn try_new(
        origin: WPoint3,
        basis: [WVec3; 2],
        outline: Vec<FacePoint>,
        holes: Vec<FaceBox>,
    ) -> Result<Self, PlanarSurfaceError> {
        let [scale_x, scale_y] = [basis[0].length(), basis[1].length()];
        ensure!(
            scale_x > MIN_EXTENT && scale_y > MIN_EXTENT,
            DegenerateBasisSnafu
        );
        let basis = [basis[0] / scale_x, basis[1] / scale_y];
        ensure!(
            basis[0].dot(basis[1]).abs() <= MAX_SKEW,
            DegenerateBasisSnafu
        );

        let outline: Vec<FacePoint> = outline
            .into_iter()
            .map(|p| FacePoint::new(p.x * scale_x, p.y * scale_y))
            .collect();
        let holes = holes
            .into_iter()
            .map(|hole| {
                FaceBox::new(
                    FacePoint::new(hole.min.x * scale_x, hole.min.y * scale_y),
                    FacePoint::new(hole.max.x * scale_x, hole.max.y * scale_y),
                )
            })
            .collect();

        let area = signed_area(&outline);
        ensure!(
            outline.len() >= 3 && area.abs() > MIN_EXTENT,
            OutlineTooSmallSnafu {
                corners: outline.len(),
                area,
            }
        );

        let mut outline = outline;
        if area < 0.0 {
            outline.reverse();
        }
        ensure_convex(&outline)?;

        Ok(Self {
            origin,
            basis,
            outline,
            holes,
        })
    }

    /// The outward normal: the side of the surface the hatching is seen from.
    pub fn normal(&self) -> WVec3 {
        self.basis[0].cross(self.basis[1])
    }

    pub fn to_world(&self, point: FacePoint) -> WPoint3 {
        self.origin + self.basis[0] * point.x + self.basis[1] * point.y
    }

    pub(crate) fn outline(&self) -> &[FacePoint] {
        &self.outline
    }

    pub(crate) fn holes(&self) -> &[FaceBox] {
        &self.holes
    }

    pub(crate) fn centroid(&self) -> WPoint3 {
        let sum = self
            .outline
            .iter()
            .fold(euclid::Vector2D::zero(), |acc, p| acc + p.to_vector());
        self.to_world((sum / self.outline.len() as f64).to_point())
    }

    /// The in-plane direction of `direction`, as an angle in this surface's
    /// frame. Directions perpendicular to the surface have no in-plane
    /// direction, and take the default diagonal.
    pub(crate) fn in_plane_angle(&self, direction: WVec3) -> Angle<f64> {
        let normal = self.normal();
        let in_plane = direction - normal * direction.dot(normal);
        if in_plane.length() < MIN_EXTENT {
            return Angle::degrees(45.0);
        }
        Angle::radians(
            in_plane
                .dot(self.basis[1])
                .atan2(in_plane.dot(self.basis[0])),
        )
    }
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

/// Twice the signed area of the polygon; positive when counter-clockwise.
fn signed_area(outline: &[FacePoint]) -> f64 {
    let count = outline.len();
    (0..count)
        .map(|ndx| {
            let a = outline[ndx];
            let b = outline[(ndx + 1) % count];
            a.x * b.y - b.x * a.y
        })
        .sum()
}

fn ensure_convex(outline: &[FacePoint]) -> Result<(), PlanarSurfaceError> {
    let count = outline.len();
    for corner in 0..count {
        let a = outline[corner];
        let b = outline[(corner + 1) % count];
        let c = outline[(corner + 2) % count];
        let turn = (b - a).cross(c - b);
        ensure!(turn >= -MIN_EXTENT, OutlineNotConvexSnafu { corner });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_square() -> Vec<FacePoint> {
        vec![
            FacePoint::new(0.0, 0.0),
            FacePoint::new(1.0, 0.0),
            FacePoint::new(1.0, 1.0),
            FacePoint::new(0.0, 1.0),
        ]
    }

    fn plane(
        basis: [WVec3; 2],
        outline: Vec<FacePoint>,
    ) -> Result<PlanarSurface, PlanarSurfaceError> {
        PlanarSurface::try_new(WPoint3::zero(), basis, outline, vec![])
    }

    fn xy_basis() -> [WVec3; 2] {
        [WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)]
    }

    #[test]
    fn the_normal_is_the_basis_cross_product() {
        let surface = plane(xy_basis(), unit_square()).expect("a unit square is a surface");
        assert_eq!(surface.normal(), WVec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn an_unnormalized_basis_rescales_the_outline() {
        let surface = plane(
            [WVec3::new(3.0, 0.0, 0.0), WVec3::new(0.0, 2.0, 0.0)],
            unit_square(),
        )
        .expect("a scaled basis still describes a surface");

        assert_eq!(surface.normal(), WVec3::new(0.0, 0.0, 1.0));
        assert_eq!(
            surface.to_world(FacePoint::new(3.0, 2.0)),
            WPoint3::new(3.0, 2.0, 0.0),
            "the outline's far corner must land where the caller placed it"
        );
        assert_eq!(surface.outline()[2], FacePoint::new(3.0, 2.0));
    }

    #[test]
    fn a_clockwise_outline_is_wound_to_match_the_normal() {
        let mut clockwise = unit_square();
        clockwise.reverse();
        let surface = plane(xy_basis(), clockwise).expect("winding is normalized, not rejected");
        assert!(signed_area(surface.outline()) > 0.0);
    }

    #[test]
    fn a_basis_which_is_not_a_frame_is_no_surface() {
        assert!(matches!(
            plane([WVec3::zero(), WVec3::new(0.0, 1.0, 0.0)], unit_square()),
            Err(PlanarSurfaceError::DegenerateBasis)
        ));
        // A skewed basis would turn rectangular holes into parallelograms.
        assert!(matches!(
            plane(
                [WVec3::new(1.0, 0.0, 0.0), WVec3::new(1.0, 1.0, 0.0)],
                unit_square()
            ),
            Err(PlanarSurfaceError::DegenerateBasis)
        ));
    }

    #[test]
    fn an_outline_enclosing_nothing_is_no_surface() {
        assert!(matches!(
            plane(
                xy_basis(),
                vec![FacePoint::new(0.0, 0.0), FacePoint::new(1.0, 0.0)]
            ),
            Err(PlanarSurfaceError::OutlineTooSmall { .. })
        ));
        assert!(matches!(
            plane(
                xy_basis(),
                vec![
                    FacePoint::new(0.0, 0.0),
                    FacePoint::new(1.0, 0.0),
                    FacePoint::new(2.0, 0.0),
                ]
            ),
            Err(PlanarSurfaceError::OutlineTooSmall { .. })
        ));
    }

    #[test]
    fn a_concave_outline_is_no_surface() {
        let arrowhead = vec![
            FacePoint::new(0.0, 0.0),
            FacePoint::new(2.0, 0.0),
            FacePoint::new(1.0, 1.0),
            FacePoint::new(2.0, 3.0),
            FacePoint::new(0.0, 3.0),
        ];
        assert!(matches!(
            plane(xy_basis(), arrowhead),
            Err(PlanarSurfaceError::OutlineNotConvex { .. })
        ));
    }

    #[test]
    fn the_centroid_of_a_square_is_its_middle() {
        let surface = plane(xy_basis(), unit_square()).expect("a unit square is a surface");
        assert_eq!(surface.centroid(), WPoint3::new(0.5, 0.5, 0.0));
    }

    #[test]
    fn an_in_plane_angle_measures_from_the_first_basis_vector() {
        let surface = plane(xy_basis(), unit_square()).expect("a unit square is a surface");
        assert_eq!(
            surface
                .in_plane_angle(WVec3::new(0.0, 1.0, 0.0))
                .to_degrees(),
            90.0
        );
        assert_eq!(
            surface
                .in_plane_angle(WVec3::new(1.0, 0.0, 4.0))
                .to_degrees(),
            0.0,
            "the component along the normal is projected away"
        );
        assert_eq!(
            surface
                .in_plane_angle(WVec3::new(0.0, 0.0, 1.0))
                .to_degrees(),
            45.0,
            "a direction with no in-plane part takes the default diagonal"
        );
    }

    #[test]
    fn a_sphere_needs_a_real_radius() {
        assert!(SphereSurface::try_new(WPoint3::zero(), 0.0).is_err());
        assert!(SphereSurface::try_new(WPoint3::zero(), -1.0).is_err());
        assert!(SphereSurface::try_new(WPoint3::zero(), f64::NAN).is_err());
        assert!(SphereSurface::try_new(WPoint3::zero(), 1.5).is_ok());
    }
}
