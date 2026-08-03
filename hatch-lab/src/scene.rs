//! The architectural test scene every hatching strategy renders.
//!
//! A gabled house with door and window openings, a chimney, a sphere-canopy
//! tree, and a ground plane, lit by a single point light. The whole scene is
//! ordinary raydeon geometry: a strategy is nothing but the hatch style its
//! material carries.
use raydeon::lights::PointLight;
use raydeon::shapes::{Quad, Sphere, Triangle};
use raydeon::{
    Camera, CollisionGeometry, DrawableShape, FaceBox, FacePoint, HatchStyle, HatchSurface,
    LineSegment3D, Material, PlanarSurface, Scene, SceneLighting, Shape, ToneWhite, WPoint3, WVec3,
    WorldSpace,
};
use std::sync::Arc;

/// The illumination this scene's light delivers to a fully lit surface.
/// Hatching normalizes tone against it, so it is tuned to the light below.
const TONE_WHITE: f64 = 1.3;

pub const RENDER_WIDTH: usize = 1024;
pub const RENDER_HEIGHT: usize = 768;

/// Where the scene's only light sits. Light-flow hatching needs to know, so
/// its strokes can follow the light across each surface.
pub fn light_position() -> WPoint3 {
    WPoint3::new(-9.0, -13.0, 13.0)
}

pub struct TestScene {
    pub scene: Scene,
    pub camera: Camera,
}

/// Builds the scene with every surface shaded by `hatch`, or with outlines
/// only when no style is given.
pub fn build(hatch: Option<HatchStyle>) -> TestScene {
    let material = Material::new()
        .diffuse(1.0)
        .specular(0.35)
        .shininess(8.0)
        .maybe_hatch(hatch)
        .build();
    let mut assembler = Assembler::new(material);

    // Ground plane.
    assembler.quad(
        (-8.0, -8.0, 0.0),
        [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
        [20.0, 15.0],
        vec![],
    );

    // House walls. Footprint x in [-4, 4], y in [-3, 3], eaves at z = 3.
    let door = opening((3.3, 0.0), (4.7, 2.2));
    let window_a = opening((0.9, 1.3), (2.1, 2.5));
    let window_b = opening((5.9, 1.3), (7.1, 2.5));
    assembler.quad(
        (-4.0, -3.0, 0.0),
        [(1.0, 0.0, 0.0), (0.0, 0.0, 1.0)],
        [8.0, 3.0],
        vec![door, window_a, window_b],
    );
    assembler.quad(
        (4.0, 3.0, 0.0),
        [(-1.0, 0.0, 0.0), (0.0, 0.0, 1.0)],
        [8.0, 3.0],
        vec![],
    );
    assembler.quad(
        (-4.0, 3.0, 0.0),
        [(0.0, -1.0, 0.0), (0.0, 0.0, 1.0)],
        [6.0, 3.0],
        vec![],
    );
    assembler.quad(
        (4.0, -3.0, 0.0),
        [(0.0, 1.0, 0.0), (0.0, 0.0, 1.0)],
        [6.0, 3.0],
        vec![opening((2.4, 1.3), (3.6, 2.5))],
    );

    // Roof planes rising from the eaves to a ridge at z = 5 along the x axis.
    let roof_slope_len = (3.0f64 * 3.0 + 2.0 * 2.0).sqrt();
    assembler.quad(
        (-4.0, -3.0, 3.0),
        [(1.0, 0.0, 0.0), (0.0, 3.0, 2.0)],
        [8.0, roof_slope_len],
        vec![],
    );
    assembler.quad(
        (4.0, 3.0, 3.0),
        [(-1.0, 0.0, 0.0), (0.0, -3.0, 2.0)],
        [8.0, roof_slope_len],
        vec![],
    );

    // Gable triangles closing the roof ends.
    assembler.gable(
        (-4.0, -3.0, 3.0),
        [(0.0, 0.0, 1.0), (0.0, 1.0, 0.0)],
        [(0.0, 0.0), (2.0, 3.0), (0.0, 6.0)],
    );
    assembler.gable(
        (4.0, -3.0, 3.0),
        [(0.0, 1.0, 0.0), (0.0, 0.0, 1.0)],
        [(0.0, 0.0), (6.0, 0.0), (3.0, 2.0)],
    );

    // Chimney rising through the north roof plane, and a tree trunk.
    assembler.cuboid((2.2, 0.8, 0.0), (3.2, 1.8, 6.0));
    assembler.cuboid((6.8, 0.8, 0.0), (7.2, 1.2, 2.4));

    // The tree canopy.
    assembler.push(Arc::new(
        Sphere::new().center((7.0, 1.0, 3.3)).radius(1.3).build(),
    ));

    let scene = Scene::new()
        .geometry(assembler.drawables)
        .lighting(
            SceneLighting::new()
                .with_lights(vec![Arc::new(
                    PointLight::new()
                        .position(light_position())
                        .intensity(2.2)
                        .specular_intensity(0.6)
                        .constant_attenuation(1.0)
                        .linear_attenuation(0.02)
                        .quadratic_attenuation(0.001)
                        .build(),
                )])
                .with_ambient_lighting(0.15)
                .with_tone_white(
                    ToneWhite::try_new(TONE_WHITE).expect("the lab's tone white is positive"),
                ),
        )
        .build();

    let camera = Camera::new()
        .observation(
            Camera::look_at(
                WPoint3::new(13.5, -10.5, 5.8),
                WVec3::new(0.5, 0.5, 2.2),
                WVec3::new(0.0, 0.0, 1.0),
            )
            .expect("the lab camera looks at a point in front of it"),
        )
        .perspective(
            Camera::perspective(42.0, RENDER_WIDTH, RENDER_HEIGHT, 0.1, 60.0)
                .expect("the lab frustum parameters are well formed"),
        )
        .build();

    TestScene { scene, camera }
}

fn opening(min: (f64, f64), max: (f64, f64)) -> FaceBox {
    FaceBox::new(FacePoint::new(min.0, min.1), FacePoint::new(max.0, max.1))
}

/// A wall with rectangular openings cut into it: the openings are drawn as
/// frames and hatching skips them, though they remain solid to collision.
#[derive(Debug)]
struct WindowedQuad {
    quad: Quad,
    openings: Vec<FaceBox>,
}

impl WindowedQuad {
    fn new(quad: Quad, openings: Vec<FaceBox>) -> Self {
        Self { quad, openings }
    }
}

impl Shape for WindowedQuad {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn CollisionGeometry>>> {
        self.quad.collision_geometry()
    }

    fn paths(&self, cam: &Camera) -> Vec<LineSegment3D<WorldSpace>> {
        // Frames sit slightly proud of the wall so they survive its occlusion.
        let lift = self.quad.basis[0].cross(self.quad.basis[1]) * 0.01;
        let corner = |x: f64, y: f64| {
            self.quad.origin + self.quad.basis[0] * x + self.quad.basis[1] * y + lift
        };

        let mut paths = self.quad.paths(cam);
        for opening in &self.openings {
            let (min, max) = (opening.min, opening.max);
            paths.extend(LineSegment3D::from_points(vec![
                (corner(min.x, min.y), corner(max.x, min.y)),
                (corner(max.x, min.y), corner(max.x, max.y)),
                (corner(max.x, max.y), corner(min.x, max.y)),
                (corner(min.x, max.y), corner(min.x, min.y)),
            ]));
        }
        paths
    }

    fn hatch_surfaces(&self) -> Vec<HatchSurface> {
        let [width, height] = self.quad.dims;
        let outline = vec![
            FacePoint::new(0.0, 0.0),
            FacePoint::new(width, 0.0),
            FacePoint::new(width, height),
            FacePoint::new(0.0, height),
        ];
        let surface = PlanarSurface::try_new(
            self.quad.origin,
            self.quad.basis,
            outline,
            self.openings.clone(),
        )
        .expect("the lab's walls are convex rectangles in an orthonormal frame");
        vec![HatchSurface::Planar(surface)]
    }
}

/// Origin, basis pair, and dimensions describing one quad to assemble.
type QuadSpec = ((f64, f64, f64), [(f64, f64, f64); 2], [f64; 2]);

/// Accumulates the scene's geometry, giving every shape the same material.
struct Assembler {
    material: Material,
    drawables: Vec<DrawableShape>,
}

impl Assembler {
    fn new(material: Material) -> Self {
        Self {
            material,
            drawables: Vec::new(),
        }
    }

    fn push(&mut self, shape: Arc<dyn Shape>) {
        self.drawables.push(
            DrawableShape::new()
                .geometry(shape)
                .material(self.material.clone())
                .build(),
        );
    }

    fn quad(
        &mut self,
        origin: (f64, f64, f64),
        basis: [(f64, f64, f64); 2],
        dims: [f64; 2],
        openings: Vec<FaceBox>,
    ) {
        let quad = Quad::new()
            .origin(point(origin))
            .basis([vector(basis[0]), vector(basis[1])])
            .dims(dims)
            .build();
        self.push(Arc::new(WindowedQuad::new(quad, openings)));
    }

    fn gable(
        &mut self,
        origin: (f64, f64, f64),
        basis: [(f64, f64, f64); 2],
        verts: [(f64, f64); 3],
    ) {
        let origin = point(origin);
        let (b0, b1) = (vector(basis[0]), vector(basis[1]));
        let world = |v: (f64, f64)| origin + b0 * v.0 + b1 * v.1;
        self.push(Arc::new(
            Triangle::new()
                .v0(world(verts[0]))
                .v1(world(verts[1]))
                .v2(world(verts[2]))
                .build(),
        ));
    }

    /// Adds the five visible faces of an axis-aligned box as quads, so each
    /// face hatches in its own plane.
    fn cuboid(&mut self, min: (f64, f64, f64), max: (f64, f64, f64)) {
        let (dx, dy, dz) = (max.0 - min.0, max.1 - min.1, max.2 - min.2);
        let sides: [QuadSpec; 5] = [
            (min, [(1.0, 0.0, 0.0), (0.0, 0.0, 1.0)], [dx, dz]),
            (
                (max.0, max.1, min.2),
                [(-1.0, 0.0, 0.0), (0.0, 0.0, 1.0)],
                [dx, dz],
            ),
            (
                (min.0, max.1, min.2),
                [(0.0, -1.0, 0.0), (0.0, 0.0, 1.0)],
                [dy, dz],
            ),
            (
                (max.0, min.1, min.2),
                [(0.0, 1.0, 0.0), (0.0, 0.0, 1.0)],
                [dy, dz],
            ),
            (
                (min.0, min.1, max.2),
                [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
                [dx, dy],
            ),
        ];
        for (origin, basis, dims) in sides {
            self.quad(origin, basis, dims, vec![]);
        }
    }
}

fn point(p: (f64, f64, f64)) -> WPoint3 {
    WPoint3::new(p.0, p.1, p.2)
}

fn vector(v: (f64, f64, f64)) -> WVec3 {
    WVec3::new(v.0, v.1, v.2).normalize()
}
