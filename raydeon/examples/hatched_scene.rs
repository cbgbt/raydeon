//! World-space hatching: a lit scene whose materials shade themselves.
//!
//! The wall is a shape of this example's own, offering a hatchable surface
//! with two openings cut into it, so hatch lines stop at the window and the
//! door while the wall itself stays solid.
use raydeon::lights::PointLight;
use raydeon::shapes::{AxisAlignedCuboid, Quad, Sphere};
use raydeon::{
    Camera, CollisionGeometry, DrawableShape, FaceBox, FacePoint, HatchSpacing, HatchStyle,
    HatchSurface, LineSegment3D, Material, PenId, PlanarSurface, Scene, SceneLighting, Shape,
    ToneWhite, WPoint3, WVec3, WorldSpace,
};
use std::sync::Arc;

const WIDTH: usize = 1024;
const HEIGHT: usize = 1024;

/// Where the light sits. Light-flow hatching follows it across a surface.
const LIGHT: (f64, f64, f64) = (-8.0, -3.0, 8.0);

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let spacing = HatchSpacing::try_new(0.22).expect("the example's spacing is positive");
    let hatched = |style: HatchStyle, pen: usize| {
        Material::new()
            .diffuse(1.0)
            .specular(0.35)
            .shininess(8.0)
            .pen(PenId::new(pen))
            .hatch(style)
            .build()
    };

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

    let scene = Scene::new()
        .geometry(vec![
            DrawableShape::new()
                .geometry(Arc::new(ground))
                .material(hatched(HatchStyle::stochastic(spacing), 0))
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(wall))
                .material(hatched(HatchStyle::tonal_crosshatch(spacing), 1))
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(
                    AxisAlignedCuboid::new()
                        .min((3.6, -2.6, 0.0))
                        .max((5.2, -1.0, 1.6))
                        .build(),
                ))
                .material(hatched(HatchStyle::tonal_crosshatch(spacing), 1))
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(
                    Sphere::new().center((-4.4, -2.6, 1.5)).radius(1.5).build(),
                ))
                .material(hatched(
                    HatchStyle::light_flow(WPoint3::new(LIGHT.0, LIGHT.1, LIGHT.2), spacing),
                    2,
                ))
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
        .expect("the example's wall is a convex rectangle in an orthonormal frame");
        vec![HatchSurface::Planar(surface)]
    }
}
