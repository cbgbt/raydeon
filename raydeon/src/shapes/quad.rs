use bon::Builder;
use std::sync::Arc;

use super::Triangle;
use crate::hatch::surface::{offer_planar, FacePoint, PlanarSurface};
use crate::path::LineSegment3D;
use crate::{Camera, CollisionGeometry, HatchSurface, Shape, WPoint3, WVec3, WorldSpace};

#[derive(Debug, Copy, Clone, Builder)]
#[builder(start_fn(name = new))]
#[cfg_attr(test, derive(PartialEq))]
pub struct Quad {
    #[builder(into)]
    pub origin: WPoint3,

    #[builder(with = |basis: impl Into<[WVec3; 2]>| {
        let basis = basis.into();
        [basis[0].normalize(), basis[1].normalize()]
    })]
    pub basis: [WVec3; 2],
    pub dims: [f64; 2],

    #[builder(skip = [
            origin,
            origin + basis[0] * dims[0],
            origin + basis[0] * dims[0] + basis[1] * dims[1],
            origin + basis[1] * dims[1],
    ])]
    pub verts: [WPoint3; 4],
}

impl Shape for Quad {
    fn collision_geometry(&self) -> Option<Vec<std::sync::Arc<dyn CollisionGeometry>>> {
        Some(vec![
            Arc::new(
                Triangle::new()
                    .v0(self.verts[0])
                    .v1(self.verts[1])
                    .v2(self.verts[3])
                    .build(),
            ),
            Arc::new(
                Triangle::new()
                    .v0(self.verts[1])
                    .v1(self.verts[2])
                    .v2(self.verts[3])
                    .build(),
            ),
        ])
    }

    fn paths(&self, _cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        let centroid = self
            .verts
            .into_iter()
            .map(WPoint3::to_vector)
            .sum::<WVec3>()
            / 4.0;

        let v = self
            .verts
            .into_iter()
            .map(|v| v + (v.to_vector() - centroid).normalize() * 0.0015)
            .collect::<Vec<_>>();

        LineSegment3D::from_points(vec![(v[0], v[1]), (v[1], v[2]), (v[2], v[3]), (v[3], v[0])])
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        let [width, height] = self.dims;
        let outline = vec![
            FacePoint::new(0.0, 0.0),
            FacePoint::new(width, 0.0),
            FacePoint::new(width, height),
            FacePoint::new(0.0, height),
        ];
        offer_planar(
            PlanarSurface::try_new(self.origin, self.basis, outline, vec![]),
            self,
        )
        .into_iter()
        .collect()
    }
}
