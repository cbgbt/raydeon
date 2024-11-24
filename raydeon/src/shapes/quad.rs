use std::sync::Arc;

use super::Triangle;
use crate::path::LineSegment3D;
use crate::{
    Camera, CollisionGeometry, Observation, PathMeta, Perspective, Shape, WPoint3, WVec3,
    WorldSpace,
};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Quad<P: PathMeta> {
    pub origin: WPoint3,
    pub basis: [WVec3; 2],
    pub dims: [f64; 2],
    pub verts: [WPoint3; 4],
    meta: P,
}

impl Quad<usize> {
    pub fn new(origin: WPoint3, basis: [WVec3; 2], dims: [f64; 2]) -> Self {
        Self::tagged(origin, basis, dims, 0)
    }
}

impl<P: PathMeta> Quad<P> {
    pub fn tagged(origin: WPoint3, mut basis: [WVec3; 2], dims: [f64; 2], meta: P) -> Quad<P> {
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
            meta,
        }
    }
}

impl<P: PathMeta> Shape<WorldSpace, P> for Quad<P> {
    fn collision_geometry(&self) -> Option<Vec<std::sync::Arc<dyn CollisionGeometry<WorldSpace>>>> {
        Some(vec![
            Arc::new(Triangle::new(self.verts[0], self.verts[1], self.verts[3])),
            Arc::new(Triangle::new(self.verts[1], self.verts[2], self.verts[3])),
        ])
    }

    fn paths(&self, _cam: &Camera<Perspective, Observation>) -> Vec<LineSegment3D<WorldSpace, P>> {
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
            LineSegment3D::tagged(v[0], v[1], self.meta.clone()),
            LineSegment3D::tagged(v[1], v[2], self.meta.clone()),
            LineSegment3D::tagged(v[2], v[3], self.meta.clone()),
            LineSegment3D::tagged(v[3], v[0], self.meta.clone()),
        ]
    }
}
