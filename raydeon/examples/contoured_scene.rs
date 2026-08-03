//! Field-contour outlines: a lathe-thrown vase whose silhouette and tone
//! bands are drawn as contours, a sphere whose shadow terminator is a
//! contour, and a windowed wall casting a shadow onto a contoured ground.
//!
//! This is a NEW example precisely so `hatched_scene`'s golden keeps pinning
//! the contour-free hatch pipeline untouched.
use raydeon::lights::PointLight;
use raydeon::shapes::{Lathe, Quad, Sphere};
use raydeon::{
    Camera, CollisionGeometry, ContourField, ContourStyle, DrawableShape, FaceBox, FacePoint,
    HatchSpacing, HatchStyle, HatchSurface, LineSegment3D, Material, PenId, PlanarSurface,
    ProfilePoint, Scene, SceneLighting, Shape, ToneThreshold, ToneWhite, WPoint3, WVec3,
    WorldSpace,
};
use std::sync::Arc;

const WIDTH: usize = 1024;
const HEIGHT: usize = 1024;

/// Where the light sits. Its shadow of the windowed wall falls across the
/// contoured ground, and its terminator on the sphere becomes a contour.
const LIGHT: (f64, f64, f64) = (-8.0, -3.0, 8.0);

/// The vase profile, foot to lip, scaled up from the storefront's tabletop
/// scale to this scene's architectural one.
const VASE_SCALE: f64 = 3.0;

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let spacing = HatchSpacing::try_new(0.22).expect("the example's spacing is positive");
    let shadow_edge = ToneThreshold::try_new(0.5).expect("0.5 is a valid tone threshold");

    let ground = Quad::new()
        .origin(WPoint3::new(-6.0, -6.0, 0.0))
        .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)])
        .dims([14.0, 12.0])
        .build();

    let wall = WindowedQuad::new(
        Quad::new()
            .origin(WPoint3::new(-3.0, 0.0, 0.0))
            .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 0.0, 1.0)])
            .dims([6.0, 4.0])
            .build(),
        vec![
            opening((0.6, 0.0), (1.9, 2.4)),
            opening((3.2, 1.2), (4.6, 2.6)),
        ],
    );

    let vase_style = HatchStyle::tonal_crosshatch(spacing);
    let mut vase_fields = ContourStyle::band_edges(&vase_style).fields().to_vec();
    vase_fields.push(ContourField::Silhouette);
    let vase_contours = ContourStyle::new().fields(vase_fields).build();

    let vase = Lathe::new()
        .base(WPoint3::new(4.4, -1.8, 0.0))
        .profile(
            [
                (0.16, 0.0),
                (0.21, 0.05),
                (0.17, 0.12),
                (0.26, 0.30),
                (0.31, 0.48),
                (0.27, 0.62),
                (0.16, 0.74),
                (0.13, 0.82),
                (0.18, 0.90),
            ]
            .map(|(radius, height)| ProfilePoint {
                radius: radius * VASE_SCALE,
                height: height * VASE_SCALE,
            })
            .to_vec(),
        )
        .build();

    let scene = Scene::new()
        .geometry(vec![
            DrawableShape::new()
                .geometry(Arc::new(ground))
                .material(
                    Material::new()
                        .diffuse(1.0)
                        .specular(0.1)
                        .shininess(4.0)
                        .pen(PenId::new(0))
                        .contours(ContourStyle::shadow(shadow_edge))
                        .build(),
                )
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(wall))
                .material(
                    Material::new()
                        .diffuse(1.0)
                        .specular(0.1)
                        .shininess(4.0)
                        .pen(PenId::new(1))
                        .build(),
                )
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(vase))
                .material(
                    Material::new()
                        .diffuse(1.0)
                        .specular(0.4)
                        .shininess(10.0)
                        .pen(PenId::new(2))
                        .hatch(vase_style)
                        .contours(vase_contours)
                        .build(),
                )
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(
                    Sphere::new().center((-4.4, -2.6, 1.5)).radius(1.5).build(),
                ))
                .material(
                    Material::new()
                        .diffuse(1.0)
                        .specular(0.35)
                        .shininess(8.0)
                        .pen(PenId::new(3))
                        .contours(ContourStyle::shadow(shadow_edge))
                        .build(),
                )
                .build(),
        ])
        .lighting(
            SceneLighting::new()
                .with_lights(vec![Arc::new(
                    PointLight::new()
                        .position(LIGHT)
                        .intensity(2.2)
                        .specular_intensity(0.6)
                        .constant_attenuation(1.0)
                        .linear_attenuation(0.02)
                        .quadratic_attenuation(0.001)
                        .build(),
                )])
                .with_ambient_lighting(0.15)
                .with_tone_white(
                    ToneWhite::try_new(1.3).expect("the example's tone white is positive"),
                ),
        )
        .build();

    let camera = Camera::new()
        .observation(
            Camera::look_at(
                WPoint3::new(12.0, -11.5, 6.5),
                WVec3::new(0.0, 0.5, 1.4),
                WVec3::new(0.0, 0.0, 1.0),
            )
            .expect("the example's camera looks at a point in front of it"),
        )
        .perspective(
            Camera::perspective(45.0, WIDTH, HEIGHT, 0.1, 40.0)
                .expect("the example's frustum parameters are well formed"),
        )
        .build();

    let rendering = scene.attach_camera(camera).render();

    let mut svg_doc = svg::Document::new()
        .set("width", "8in")
        .set("height", "8in")
        .set("viewBox", (0, 0, WIDTH, HEIGHT))
        .set("stroke-width", "0.5mm")
        .set("stroke", "black")
        .set("fill", "none")
        .add(
            svg::node::element::Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", "100%")
                .set("height", "100%")
                .set("fill", "white"),
        );

    // We have to flip the y-axis in our svg...
    let mut item_group = svg::node::element::Group::new()
        .set("transform", format!("translate(0, {}) scale(1,-1)", HEIGHT));

    for stroke in rendering.strokes() {
        let (p1, p2) = (stroke.p1, stroke.p2);
        item_group = item_group.add(
            svg::node::element::Line::new()
                .set("x1", p1.x)
                .set("y1", p1.y)
                .set("x2", p2.x)
                .set("y2", p2.y),
        );
    }

    svg_doc = svg_doc.add(item_group);
    println!("{}", svg_doc);
}

fn opening(min: (f64, f64), max: (f64, f64)) -> FaceBox {
    FaceBox::new(FacePoint::new(min.0, min.1), FacePoint::new(max.0, max.1))
}

/// A wall with rectangular openings cut into it: the openings are drawn as
/// frames and hatching skips them, though they remain solid to collision —
/// so its cast shadow lands on the ground with two bites taken out of it.
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
        .expect("the example's wall is a convex rectangle in an orthonormal frame");
        vec![HatchSurface::Planar(surface)]
    }
}
