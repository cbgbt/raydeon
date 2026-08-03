use dot_vox;
use euclid::Angle;
use raydeon::lights::PointLight;
use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{Camera, CameraOptions, DrawableShape, Material, Scene, SceneLighting};
use std::sync::Arc;

const CASTLE_VOX: &[u8] = include_bytes!("./assets/castle.vox");

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let castle_vox = dot_vox::load_bytes(CASTLE_VOX).expect("Could not load castle.vox");

    let geometry: Vec<_> = castle_vox.models[0]
        .voxels
        .iter()
        .map(|v| {
            let x = v.x as f64;
            let y = v.y as f64;
            let z = v.z as f64;
            DrawableShape::new()
                .geometry(Arc::new(
                    AxisAlignedCuboid::new()
                        .min((x + 0.05, y + 0.05, z + 0.05))
                        .max((x + 0.95, y + 0.95, z + 0.95))
                        .build(),
                ))
                .material(Material::new().diffuse(5.0).build())
                .build()
        })
        .collect();

    let eye = (10.0, -20.0, 0.0);
    let focus = (10.0, 0.0, 0.0);
    let up = (0.0, 0.0, 1.0);

    let fovy = 40.0;
    let width = 2048;
    let height = 2048;
    let znear = 0.1;
    let zfar = 200.0;

    let mut camera = Camera::configure()
        .observation(Camera::look_at(eye, focus, up))
        .perspective(Camera::perspective(fovy, width, height, znear, zfar))
        .render_options(CameraOptions::configure().pen_px_size(4.0).build())
        .build();

    camera.translate((-15.0, 10.75, 0.0));
    camera.adjust_yaw(Angle::degrees(-45.0));
    camera.translate((-10.5, 0.0, 12.0));

    let scene = Scene::new()
        .geometry(geometry)
        .lighting(
            SceneLighting::new()
                .with_ambient_lighting(0.37)
                .with_lights(vec![Arc::new(PointLight::new(
                    55.0,
                    10.0,
                    (-10.81, -20.0, 30.0),
                    0.0,
                    0.13,
                    0.19,
                ))]),
        )
        .construct();

    let rendering = scene
        .attach_camera(camera)
        .with_seed(0)
        .render_with_lighting();

    let mut svg_doc = svg::Document::new()
        .set("width", "8in")
        .set("height", "8in")
        .set("viewBox", (0, 0, width, height))
        .set("stroke-width", "0.7mm")
        .set("stroke", "black")
        .set("fill", "none")
        .add(
            svg::node::element::Group::new().add(
                svg::node::element::Rectangle::new()
                    .set("x", 0)
                    .set("y", 0)
                    .set("width", "100%")
                    .set("height", "100%")
                    .set("fill", "white"),
            ),
        );

    // We have to flip the y-axis in our svg...
    let mut item_group = svg::node::element::Group::new()
        .set("transform", format!("translate(0, {}) scale(1,-1)", height));

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
