use std::sync::Arc;

use super::Triangle;
use crate::path::LineSegment3D;
use crate::{
    Camera, CollisionGeometry, Observation, Perspective, Shape, WPoint3, WVec3, WorldSpace,
};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Quad {
    pub origin: WPoint3,
    pub basis: [WVec3; 2],
    pub dims: [f64; 2],
    pub verts: [WPoint3; 4],
    tag: usize,
}

impl Quad {
    pub fn new(origin: WPoint3, basis: [WVec3; 2], dims: [f64; 2]) -> Quad {
        Self::tagged(origin, basis, dims, 0)
    }

    pub fn tagged(origin: WPoint3, mut basis: [WVec3; 2], dims: [f64; 2], tag: usize) -> Quad {
        basis[0] = basis[0].normalize();
        basis[1] = basis[1].normalize();
        let verts = [
            origin,
            origin + basis[0] * dims[0],
            origin + basis[0] * dims[0] + basis[1] * dims[1],
            origin + basis[1] * dims[1],
        ];
        Quad {
            origin,
            basis,
            dims,
            verts,
            tag,
        }
    }
}

impl Shape<WorldSpace> for Quad {
    fn collision_geometry(&self) -> Option<Vec<std::sync::Arc<dyn CollisionGeometry<WorldSpace>>>> {
        Some(vec![
            Arc::new(Triangle::new(self.verts[0], self.verts[1], self.verts[3])),
            Arc::new(Triangle::new(self.verts[1], self.verts[2], self.verts[3])),
        ])
    }

    fn paths(&self, _cam: &Camera<Perspective, Observation>) -> Vec<LineSegment3D<WorldSpace>> {
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

        vec![
            LineSegment3D::tagged(v[0], v[1], self.tag),
            LineSegment3D::tagged(v[1], v[2], self.tag),
            LineSegment3D::tagged(v[2], v[3], self.tag),
            LineSegment3D::tagged(v[3], v[0], self.tag),
        ]
    }
}
