//! Wheel-thrown forms: a radial profile spun around a vertical axis.
use bon::Builder;
use std::sync::Arc;

use super::triangle::Triangle;
use crate::hatch::surface::{offer_revolution, ProfilePoint, RevolutionSurface};
use crate::path::LineSegment3D;
use crate::{Camera, CollisionGeometry, HatchSurface, Shape, WPoint3, WVec3, WorldSpace};

/// How many facets the throwing rings and collision bands are built from.
///
/// Precedent: [`super::sphere::Sphere`]'s `CONTOUR_STEPS`.
const SEGMENTS: usize = 28;

/// A throwing ring is drawn this far outside the collision surface so the
/// drawn arc survives its own occlusion check.
const RING_LIFT: f64 = 0.008;

/// A surface of revolution, wheel-thrown and standing upright: a lathe is
/// always spun around +Z, the axis it stands on. A shape wanting a tilted
/// or arbitrary axis reaches for [`RevolutionSurface`] directly.
#[derive(Debug, Clone, Builder)]
#[builder(start_fn(name = new))]
pub struct Lathe {
    /// Where the axis meets what the piece stands on.
    #[builder(into)]
    base: WPoint3,
    /// The profile, foot to lip.
    profile: Vec<ProfilePoint>,
}

impl Lathe {
    fn ring_point(&self, radius: f64, height: f64, ndx: usize) -> WPoint3 {
        let angle = (ndx % SEGMENTS) as f64 / SEGMENTS as f64 * std::f64::consts::TAU;
        self.base + WVec3::new(radius * angle.cos(), radius * angle.sin(), height)
    }
}

impl Shape for Lathe {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        let mut triangles: Vec<Arc<dyn CollisionGeometry>> = Vec::new();
        for band in self.profile.windows(2) {
            let (a, b) = (band[0], band[1]);
            for ndx in 0..SEGMENTS {
                let a0 = self.ring_point(a.radius, a.height, ndx);
                let a1 = self.ring_point(a.radius, a.height, ndx + 1);
                let b0 = self.ring_point(b.radius, b.height, ndx);
                let b1 = self.ring_point(b.radius, b.height, ndx + 1);
                triangles.push(Arc::new(Triangle::new().v0(a0).v1(a1).v2(b0).build()));
                triangles.push(Arc::new(Triangle::new().v0(b0).v1(a1).v2(b1).build()));
            }
        }
        Some(triangles)
    }

    fn paths(&self, _cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        // Throwing rings at each profile step, lifted off the surface so the
        // visible arcs survive their own occlusion check.
        let mut rings = Vec::new();
        for point in &self.profile {
            let lifted = point.radius + RING_LIFT;
            for ndx in 0..SEGMENTS {
                rings.push(LineSegment3D::new_segment(
                    self.ring_point(lifted, point.height, ndx),
                    self.ring_point(lifted, point.height, ndx + 1),
                ));
            }
        }
        rings
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        let axis = WVec3::new(0.0, 0.0, 1.0);
        offer_revolution(
            RevolutionSurface::try_new(self.base, axis, self.profile.clone()),
            self,
        )
        .into_iter()
        .collect()
    }
}
