//! The architectural test scene every hatching strategy renders.
//!
//! A gabled house with door and window openings, a chimney, a sphere-canopy
//! tree, and a ground plane, lit by a single point light. Alongside the
//! raydeon [`Scene`] this keeps per-face metadata (plane basis, outline
//! polygon, holes) that world-space hatching strategies need.
use crate::shapes::{BallShape, Decal};
use raydeon::lights::PointLight;
use raydeon::shapes::{Quad, Sphere, Triangle};
use raydeon::{Camera, DrawableShape, Material, Scene, SceneLighting, Shape, WPoint3, WVec3};
use std::sync::Arc;

/// 2D coordinates within a face's plane, in units of the face basis vectors.
#[derive(Debug, Copy, Clone)]
pub struct FaceSpace;

pub type FacePoint = euclid::Point2D<f64, FaceSpace>;
pub type FaceBox = euclid::Box2D<f64, FaceSpace>;

/// Illumination value at which a surface reads as fully white.
pub const TONE_WHITE: f64 = 1.3;

pub const RENDER_WIDTH: usize = 1024;
pub const RENDER_HEIGHT: usize = 768;

/// A planar convex region of scene geometry that can carry hatch lines.
#[derive(Debug, Clone)]
pub struct Face {
    /// Shares its geometry `Arc` with the shape registered in the scene, so
    /// illumination shadow tests recognize hits on this face as unoccluded.
    pub drawable: DrawableShape,
    pub origin: WPoint3,
    /// Unit in-plane basis; `basis[0] x basis[1]` equals the outward normal.
    pub basis: [WVec3; 2],
    pub normal: WVec3,
    /// Convex outline in face coordinates, counter-clockwise.
    pub outline: Vec<FacePoint>,
    /// Openings (door, windows) that hatch lines must skip.
    pub holes: Vec<FaceBox>,
}

impl Face {
    pub fn to_world(&self, p: FacePoint) -> WPoint3 {
        self.origin + self.basis[0] * p.x + self.basis[1] * p.y
    }
}

/// A sphere in the scene, hatched with contour rings rather than plane lines.
#[derive(Debug, Clone)]
pub struct Ball {
    pub drawable: DrawableShape,
    pub center: WPoint3,
    pub radius: f64,
}

pub struct TestScene {
    pub scene: Scene,
    pub faces: Vec<Face>,
    pub balls: Vec<Ball>,
    pub camera: Camera,
    pub light_position: WPoint3,
}

pub fn build() -> TestScene {
    let mut assembler = Assembler::new(
        Material::new()
            .diffuse(1.0)
            .specular(0.35)
            .shininess(8.0)
            .build(),
    );

    // Ground plane.
    assembler.quad(
        (-8.0, -8.0, 0.0),
        [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
        [20.0, 15.0],
        vec![],
    );

    // House walls. Footprint x in [-4, 4], y in [-3, 3], eaves at z = 3.
    let door = FaceBox::new(FacePoint::new(3.3, 0.0), FacePoint::new(4.7, 2.2));
    let window_a = FaceBox::new(FacePoint::new(0.9, 1.3), FacePoint::new(2.1, 2.5));
    let window_b = FaceBox::new(FacePoint::new(5.9, 1.3), FacePoint::new(7.1, 2.5));
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
    let side_window = FaceBox::new(FacePoint::new(2.4, 1.3), FacePoint::new(3.6, 2.5));
    assembler.quad(
        (4.0, -3.0, 0.0),
        [(0.0, 1.0, 0.0), (0.0, 0.0, 1.0)],
        [6.0, 3.0],
        vec![side_window],
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

    let balls = vec![assembler.ball((7.0, 1.0, 3.3), 1.3)];

    // Door and window frames: drawn outlines with no collision geometry.
    assembler.frame_holes_of_face(1);
    assembler.frame_holes_of_face(4);

    let light_position = WPoint3::new(-9.0, -13.0, 13.0);
    let scene = Scene::new()
        .geometry(assembler.drawables)
        .lighting(
            SceneLighting::new()
                .with_lights(vec![Arc::new(
                    PointLight::new()
                        .position(light_position)
                        .intensity(2.2)
                        .specular_intensity(0.6)
                        .constant_attenuation(1.0)
                        .linear_attenuation(0.02)
                        .quadratic_attenuation(0.001)
                        .build(),
                )])
                .with_ambient_lighting(0.15),
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

    TestScene {
        scene,
        faces: assembler.faces,
        balls,
        camera,
        light_position,
    }
}

/// Origin, basis pair, and dimensions describing one quad to assemble.
type QuadSpec = ((f64, f64, f64), [(f64, f64, f64); 2], [f64; 2]);

/// Accumulates scene geometry while recording hatchable face metadata.
struct Assembler {
    material: Material,
    faces: Vec<Face>,
    drawables: Vec<DrawableShape>,
}

impl Assembler {
    fn new(material: Material) -> Self {
        Self {
            material,
            faces: Vec::new(),
            drawables: Vec::new(),
        }
    }

    fn register(&mut self, shape: Arc<dyn Shape>) -> DrawableShape {
        let drawable = DrawableShape::new()
            .geometry(shape)
            .material(self.material)
            .build();
        self.drawables.push(drawable.clone());
        drawable
    }

    fn quad(
        &mut self,
        origin: (f64, f64, f64),
        basis: [(f64, f64, f64); 2],
        dims: [f64; 2],
        holes: Vec<FaceBox>,
    ) {
        let origin = WPoint3::new(origin.0, origin.1, origin.2);
        let b0 = WVec3::new(basis[0].0, basis[0].1, basis[0].2).normalize();
        let b1 = WVec3::new(basis[1].0, basis[1].1, basis[1].2).normalize();
        let quad = Quad::new()
            .origin(origin)
            .basis([b0, b1])
            .dims(dims)
            .build();
        let drawable = self.register(Arc::new(quad));
        self.faces.push(Face {
            drawable,
            origin,
            basis: [b0, b1],
            normal: b0.cross(b1),
            outline: vec![
                FacePoint::new(0.0, 0.0),
                FacePoint::new(dims[0], 0.0),
                FacePoint::new(dims[0], dims[1]),
                FacePoint::new(0.0, dims[1]),
            ],
            holes,
        });
    }

    fn gable(
        &mut self,
        origin: (f64, f64, f64),
        basis: [(f64, f64, f64); 2],
        verts: [(f64, f64); 3],
    ) {
        let origin = WPoint3::new(origin.0, origin.1, origin.2);
        let b0 = WVec3::new(basis[0].0, basis[0].1, basis[0].2).normalize();
        let b1 = WVec3::new(basis[1].0, basis[1].1, basis[1].2).normalize();
        let world = |v: (f64, f64)| origin + b0 * v.0 + b1 * v.1;
        let tri = Triangle::new()
            .v0(world(verts[0]))
            .v1(world(verts[1]))
            .v2(world(verts[2]))
            .build();
        let drawable = self.register(Arc::new(tri));
        self.faces.push(Face {
            drawable,
            origin,
            basis: [b0, b1],
            normal: b0.cross(b1),
            outline: verts.iter().map(|v| FacePoint::new(v.0, v.1)).collect(),
            holes: vec![],
        });
    }

    /// Adds the five visible faces of an axis-aligned cuboid as quads.
    fn cuboid(&mut self, min: (f64, f64, f64), max: (f64, f64, f64)) {
        let (dx, dy, dz) = (max.0 - min.0, max.1 - min.1, max.2 - min.2);
        let sides: [QuadSpec; 5] = [
            (
                (min.0, min.1, min.2),
                [(1.0, 0.0, 0.0), (0.0, 0.0, 1.0)],
                [dx, dz],
            ),
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

    fn ball(&mut self, center: (f64, f64, f64), radius: f64) -> Ball {
        let center = WPoint3::new(center.0, center.1, center.2);
        let shape = BallShape::new(Sphere::new().center(center).radius(radius).build());
        let drawable = self.register(Arc::new(shape));
        Ball {
            drawable,
            center,
            radius,
        }
    }

    fn frame_holes_of_face(&mut self, face_ndx: usize) {
        let face = self.faces[face_ndx].clone();
        for hole in face.holes.clone() {
            let has_muntins = hole.min.y > 0.5;
            let decal = Decal::new(face.clone(), hole, has_muntins);
            self.register(Arc::new(decal));
        }
    }
}
