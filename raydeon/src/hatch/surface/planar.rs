//! Convex planar patches: flat faces, optionally with rectangular openings.

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

/// Why a description of a planar surface describes no surface.
#[derive(Debug, Snafu)]
pub enum PlanarSurfaceError {
    #[snafu(display(
        "the basis vectors must be non-zero and non-parallel to describe the frame the \
         outline is written in"
    ))]
    DegenerateBasis,
    #[snafu(display(
        "the basis vectors are skewed, which would deform rectangular holes into shapes a \
         FaceBox cannot represent; use a perpendicular basis, or drop the holes"
    ))]
    SkewedBasisWithHoles,
    #[snafu(display("an outline needs at least three corners enclosing some area, but had {corners} corners enclosing {area}"))]
    OutlineTooSmall { corners: usize, area: f64 },
    #[snafu(display("the outline turns back on itself at corner {corner}, and hatch lines can only be clipped to a convex outline"))]
    OutlineNotConvex { corner: usize },
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
    /// The basis is Gram-Schmidt orthonormalized (`e0` along `basis[0]`; `e1`
    /// from `basis[1]` with its `e0` component removed) and the outline is
    /// re-expressed EXACTLY in the orthonormal frame, so callers may describe
    /// a face with any non-degenerate basis — including a skewed (non-
    /// perpendicular) one — and the outline still lands at the world
    /// positions they intended. A clockwise outline is reversed to
    /// counter-clockwise, which is the winding the outward normal
    /// `e0.cross(e1)` implies.
    ///
    /// Holes are axis-aligned [`FaceBox`]es in the caller's frame; a skewed
    /// basis would shear them into parallelograms a `FaceBox` cannot
    /// represent, so a skewed basis combined with holes is rejected as
    /// [`PlanarSurfaceError::SkewedBasisWithHoles`] rather than silently
    /// deformed.
    pub fn try_new(
        origin: WPoint3,
        basis: [WVec3; 2],
        outline: Vec<FacePoint>,
        holes: Vec<FaceBox>,
    ) -> Result<Self, PlanarSurfaceError> {
        let scale_x = basis[0].length();
        let len_b1 = basis[1].length();
        ensure!(
            scale_x > MIN_EXTENT && len_b1 > MIN_EXTENT,
            DegenerateBasisSnafu
        );
        let e0 = basis[0] / scale_x;
        let b1_along_e0 = basis[1].dot(e0);
        let skew = (b1_along_e0 / len_b1).abs();
        ensure!(
            skew <= MAX_SKEW || holes.is_empty(),
            SkewedBasisWithHolesSnafu
        );

        let e1_raw = basis[1] - e0 * b1_along_e0;
        let scale_y = e1_raw.length();
        ensure!(scale_y > MIN_EXTENT, DegenerateBasisSnafu);
        let e1 = e1_raw / scale_y;
        let basis = [e0, e1];

        // (x, y) ↦ (x·|b0| + y·(b1·e0), y·|b1 − (b1·e0)e0|): re-expresses the
        // caller's frame exactly in the orthonormal one; b1_along_e0 = 0 (the
        // perpendicular case) degenerates to independent per-axis scaling.
        let re_express =
            |p: FacePoint| FacePoint::new(p.x * scale_x + p.y * b1_along_e0, p.y * scale_y);

        let outline: Vec<FacePoint> = outline.into_iter().map(re_express).collect();
        let holes = holes
            .into_iter()
            .map(|hole| FaceBox::new(re_express(hole.min), re_express(hole.max)))
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
    fn a_degenerate_basis_is_no_surface() {
        assert!(matches!(
            plane([WVec3::zero(), WVec3::new(0.0, 1.0, 0.0)], unit_square()),
            Err(PlanarSurfaceError::DegenerateBasis)
        ));
        // basis[1] parallel to basis[0]: nothing left after removing the e0 component.
        assert!(matches!(
            plane(
                [WVec3::new(1.0, 0.0, 0.0), WVec3::new(2.0, 0.0, 0.0)],
                unit_square()
            ),
            Err(PlanarSurfaceError::DegenerateBasis)
        ));
    }

    #[test]
    fn a_skewed_hole_less_basis_is_gram_schmidted_and_the_outline_re_expressed_exactly() {
        // A rhombus basis: not perpendicular, but a legal frame absent holes.
        let rhombus = [WVec3::new(1.0, 0.0, 0.0), WVec3::new(1.0, 1.0, 0.0)];
        let surface = plane(rhombus, unit_square())
            .expect("a skewed hole-less basis is accepted, not rejected");

        // The outward normal is preserved: e0 x e1 is parallel to b0 x b1 (both +Z here).
        assert_eq!(surface.normal(), WVec3::new(0.0, 0.0, 1.0));

        // Every outline corner must land at the exact world position the caller's
        // (skewed) basis implies: origin + x*b0 + y*b1.
        for (face, world) in unit_square().into_iter().zip([
            WPoint3::new(0.0, 0.0, 0.0),
            WPoint3::new(1.0, 0.0, 0.0),
            WPoint3::new(2.0, 1.0, 0.0),
            WPoint3::new(1.0, 1.0, 0.0),
        ]) {
            let expected = WPoint3::zero() + rhombus[0] * face.x + rhombus[1] * face.y;
            assert_eq!(expected, world, "test fixture sanity check");
        }
        for (corner, expected_world) in surface.outline().iter().zip([
            WPoint3::new(0.0, 0.0, 0.0),
            WPoint3::new(1.0, 0.0, 0.0),
            WPoint3::new(2.0, 1.0, 0.0),
            WPoint3::new(1.0, 1.0, 0.0),
        ]) {
            assert_eq!(surface.to_world(*corner), expected_world);
        }
    }

    #[test]
    fn a_skewed_basis_with_holes_is_no_surface() {
        let rhombus = [WVec3::new(1.0, 0.0, 0.0), WVec3::new(1.0, 1.0, 0.0)];
        let hole = FaceBox::new(FacePoint::new(0.25, 0.25), FacePoint::new(0.75, 0.75));
        assert!(matches!(
            PlanarSurface::try_new(WPoint3::zero(), rhombus, unit_square(), vec![hole]),
            Err(PlanarSurfaceError::SkewedBasisWithHoles)
        ));
    }

    #[test]
    fn a_perpendicular_basis_with_holes_is_unaffected_by_the_skew_gate() {
        let hole = FaceBox::new(FacePoint::new(0.25, 0.25), FacePoint::new(0.75, 0.75));
        let surface =
            PlanarSurface::try_new(WPoint3::zero(), xy_basis(), unit_square(), vec![hole])
                .expect("a perpendicular basis with holes is a surface");
        assert_eq!(
            surface.holes()[0],
            FaceBox::new(FacePoint::new(0.25, 0.25), FacePoint::new(0.75, 0.75))
        );
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
}
