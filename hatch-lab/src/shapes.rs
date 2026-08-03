//! Lab-only shapes: a sphere that draws its silhouette, and collision-free
//! decorative outlines (door and window frames).
use crate::scene::{Face, FaceBox, FacePoint};
use raydeon::shapes::Sphere;
use raydeon::{Camera, CollisionGeometry, LineSegment3D, Shape, WVec3, WorldSpace};
use std::sync::Arc;

/// A sphere that draws its view-dependent silhouette contour.
#[derive(Debug)]
pub struct BallShape {
    sphere: Sphere,
}

impl BallShape {
    pub fn new(sphere: Sphere) -> Self {
        Self { sphere }
    }
}

impl Shape for BallShape {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        Some(vec![Arc::new(self.sphere)])
    }

    fn paths(&self, cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        let center = self.sphere.center;
        // Inflate slightly so the contour survives its own occlusion check.
        let radius = self.sphere.radius + 0.006;
        let eye = cam.observation.eye();
        let to_eye = eye - center;
        let dist = to_eye.length();
        if dist <= radius {
            return Vec::new();
        }

        // The visible contour of a sphere is a circle offset toward the eye.
        let w = to_eye / dist;
        let contour_center = center + w * (radius * radius / dist);
        let contour_radius = radius * (1.0 - (radius / dist).powi(2)).sqrt();
        let seed = if w.x.abs() < 0.9 {
            WVec3::new(1.0, 0.0, 0.0)
        } else {
            WVec3::new(0.0, 1.0, 0.0)
        };
        let u = w.cross(seed).normalize();
        let v = w.cross(u);

        let steps = 96;
        let ring_point = |ndx: usize| {
            let angle = (ndx % steps) as f64 / steps as f64 * std::f64::consts::TAU;
            contour_center + u * (contour_radius * angle.cos()) + v * (contour_radius * angle.sin())
        };
        (0..steps)
            .map(|ndx| LineSegment3D::new_segment(ring_point(ndx), ring_point(ndx + 1)))
            .collect()
    }
}

/// A rectangle drawn on a face (door or window frame) with no collision.
#[derive(Debug)]
pub struct Decal {
    face: Face,
    rect: FaceBox,
    muntins: bool,
}

impl Decal {
    pub fn new(face: Face, rect: FaceBox, muntins: bool) -> Self {
        Self {
            face,
            rect,
            muntins,
        }
    }
}

impl Shape for Decal {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        None
    }

    fn paths(&self, _cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        // Push the frame slightly off the wall so it survives occlusion.
        let lift = self.face.normal * 0.01;
        let corner = |x: f64, y: f64| self.face.to_world(FacePoint::new(x, y)) + lift;
        let (min, max) = (self.rect.min, self.rect.max);
        let mut segments = vec![
            (corner(min.x, min.y), corner(max.x, min.y)),
            (corner(max.x, min.y), corner(max.x, max.y)),
            (corner(max.x, max.y), corner(min.x, max.y)),
            (corner(min.x, max.y), corner(min.x, min.y)),
        ];
        if self.muntins {
            let mid_x = (min.x + max.x) / 2.0;
            let mid_y = (min.y + max.y) / 2.0;
            segments.push((corner(mid_x, min.y), corner(mid_x, max.y)));
            segments.push((corner(min.x, mid_y), corner(max.x, mid_y)));
        }
        segments
            .into_iter()
            .map(|(p1, p2)| LineSegment3D::new_segment(p1, p2))
            .collect()
    }
}
