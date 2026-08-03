//! Detail shapes for the storefront: wheel-thrown pottery, an awning with
//! drawn stripes, and sketch-line dressing (pavement, bricks, cracks).
use raydeon::shapes::{Quad, Triangle};
use raydeon::{
    Camera, CollisionGeometry, HatchSurface, LineSegment3D, Shape, WPoint3, WVec3, WorldSpace,
};
use std::sync::Arc;

/// A surface of revolution: a radial profile spun around a vertical axis.
/// Draws its throwing rings; collision is a triangle band per profile step.
#[derive(Debug)]
pub struct Lathe {
    /// Where the axis meets the surface the piece stands on.
    base: WPoint3,
    /// `(radius, height)` pairs from foot to lip, in world units.
    profile: Vec<(f64, f64)>,
    segments: usize,
}

impl Lathe {
    pub fn new(base: WPoint3, profile: Vec<(f64, f64)>) -> Self {
        Self {
            base,
            profile,
            segments: 28,
        }
    }

    fn ring_point(&self, radius: f64, height: f64, ndx: usize) -> WPoint3 {
        let angle = (ndx % self.segments) as f64 / self.segments as f64 * std::f64::consts::TAU;
        self.base + WVec3::new(radius * angle.cos(), radius * angle.sin(), height)
    }
}

impl Shape for Lathe {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        let mut triangles: Vec<Arc<dyn CollisionGeometry>> = Vec::new();
        for band in self.profile.windows(2) {
            let ((r0, h0), (r1, h1)) = (band[0], band[1]);
            for ndx in 0..self.segments {
                let a0 = self.ring_point(r0, h0, ndx);
                let a1 = self.ring_point(r0, h0, ndx + 1);
                let b0 = self.ring_point(r1, h1, ndx);
                let b1 = self.ring_point(r1, h1, ndx + 1);
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
        for &(radius, height) in &self.profile {
            let lifted = radius + 0.008;
            for ndx in 0..self.segments {
                rings.push(LineSegment3D::new_segment(
                    self.ring_point(lifted, height, ndx),
                    self.ring_point(lifted, height, ndx + 1),
                ));
            }
        }
        rings
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        Vec::new()
    }
}

/// The tall vase profile, foot to lip.
pub fn vase(base: WPoint3) -> Lathe {
    Lathe::new(
        base,
        vec![
            (0.16, 0.0),
            (0.21, 0.05),
            (0.17, 0.12),
            (0.26, 0.30),
            (0.31, 0.48),
            (0.27, 0.62),
            (0.16, 0.74),
            (0.13, 0.82),
            (0.18, 0.90),
        ],
    )
}

/// A wide, low bowl.
pub fn bowl(base: WPoint3) -> Lathe {
    Lathe::new(
        base,
        vec![
            (0.11, 0.0),
            (0.27, 0.06),
            (0.36, 0.16),
            (0.38, 0.25),
            (0.34, 0.29),
        ],
    )
}

/// A sloped awning canvas whose stripes are drawn geometry, not hatching.
#[derive(Debug)]
pub struct Awning {
    quad: Quad,
    stripe_step: f64,
}

impl Awning {
    pub fn new(origin: WPoint3, slope: WVec3, width: f64, stripe_step: f64) -> Self {
        let quad = Quad::new()
            .origin(origin)
            .basis([slope.normalize(), WVec3::new(1.0, 0.0, 0.0)])
            .dims([slope.length(), width])
            .build();
        Self { quad, stripe_step }
    }
}

impl Shape for Awning {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        self.quad.collision_geometry()
    }

    fn paths(&self, cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        let lift = self.quad.basis[0].cross(self.quad.basis[1]) * 0.008;
        let [slope_len, width] = self.quad.dims;
        let mut paths = self.quad.paths(cam);
        let mut v = self.stripe_step;
        while v < width {
            let top = self.quad.origin + self.quad.basis[1] * v + lift;
            let bottom = top + self.quad.basis[0] * slope_len;
            paths.push(LineSegment3D::new_segment(top, bottom));
            v += self.stripe_step;
        }
        paths
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        self.quad.hatch_surfaces()
    }
}

/// Pure drawn linework: no collision, no hatching, always plotted.
#[derive(Debug)]
pub struct SketchLines(Vec<LineSegment3D<WorldSpace>>);

impl Shape for SketchLines {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        None
    }

    fn paths(&self, _cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        self.0.clone()
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        Vec::new()
    }
}

/// Sidewalk slab joints stopped at the curb, plus the curb's two edge lines.
pub fn pavement() -> SketchLines {
    let mut joints = Vec::new();
    let mut x = -8.4;
    while x < 7.5 {
        joints.push(LineSegment3D::new_segment(
            WPoint3::new(x, -3.2, 0.004),
            WPoint3::new(x, -0.02, 0.004),
        ));
        x += 1.6;
    }
    for y in [-3.2, -3.38] {
        joints.push(LineSegment3D::new_segment(
            WPoint3::new(-9.3, y, 0.004),
            WPoint3::new(7.8, y, 0.004),
        ));
    }
    SketchLines(joints)
}

/// Weathering cracks wandering out of the sidewalk joints.
pub fn cracks() -> SketchLines {
    let ground = |points: &[(f64, f64)]| {
        points
            .windows(2)
            .map(|pair| {
                LineSegment3D::new_segment(
                    WPoint3::new(pair[0].0, pair[0].1, 0.004),
                    WPoint3::new(pair[1].0, pair[1].1, 0.004),
                )
            })
            .collect::<Vec<_>>()
    };
    let mut lines = ground(&[
        (-1.6, -1.35),
        (-1.35, -1.1),
        (-1.4, -0.85),
        (-1.15, -0.6),
        (-1.2, -0.35),
    ]);
    lines.extend(ground(&[
        (3.6, -2.6),
        (3.85, -2.3),
        (3.8, -2.05),
        (4.1, -1.85),
        (4.05, -1.6),
    ]));
    SketchLines(lines)
}

/// Distressed patches where brick coursing shows through the render's plaster:
/// short runs of courses with staggered head joints, at chosen facade spots.
pub fn brick_patches() -> SketchLines {
    const COURSE: f64 = 0.13;
    const BRICK: f64 = 0.30;
    // (patch center x/z on the y=0 facade, rows, row half-widths)
    let patches: [((f64, f64), &[f64]); 4] = [
        ((-4.35, 0.55), &[0.45, 0.62, 0.5, 0.3]),
        ((-1.55, 3.6), &[0.35, 0.5, 0.42]),
        ((4.45, 0.9), &[0.4, 0.55, 0.48, 0.28]),
        ((1.4, 4.05), &[0.5, 0.66, 0.4]),
    ];

    let mut lines = Vec::new();
    for ((cx, cz), half_widths) in patches {
        for (row, half) in half_widths.iter().enumerate() {
            let z = cz + row as f64 * COURSE;
            lines.push(LineSegment3D::new_segment(
                WPoint3::new(cx - half, -0.008, z),
                WPoint3::new(cx + half, -0.008, z),
            ));
            // Staggered head joints between this course and the one above.
            let offset = if row % 2 == 0 { 0.0 } else { BRICK / 2.0 };
            let mut x = cx - half + offset + BRICK * 0.25;
            while x < cx + half - 0.05 {
                lines.push(LineSegment3D::new_segment(
                    WPoint3::new(x, -0.008, z),
                    WPoint3::new(x, -0.008, z + COURSE * 0.85),
                ));
                x += BRICK;
            }
        }
    }
    SketchLines(lines)
}
