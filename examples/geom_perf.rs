use anyhow::{Context, Result};
use raydeon::shapes::RectPrism;
use raydeon::{Camera, Scene, Shape, WPoint3, WVec3, WorldSpace};

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let eye = WPoint3::new(-5.0, 8.0, -6.0);
    let focus = WVec3::new(10.0, 0.0, 10.0);

    let look = (eye.to_vector() - focus).normalize();
    let up = look.cross(WVec3::new(0.0, 1.0, 0.0)).cross(look);

    let fovy = 50.0;
    let width = 1024.0;
    let height = 1024.0;
    let znear = 0.1;
    let zfar = 100.0;

    let scene = Scene::new(generate_scene());

    let camera = Camera::look_at(eye, focus, up).perspective(fovy, width, height, znear, zfar);

    let paths = scene.attach_camera(camera).render();

    // We currently don't have any functionality to aid in emitting SVG images, so you will
    // be required to use the [svg crate.](https://crates.io/crates/svg)
    let mut svg_doc = svg::Document::new()
        .set("width", "8in")
        .set("height", "8in")
        .set("viewBox", (0, 0, width, height))
        .set("stroke-width", "0.3mm")
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
        .set("transform", format!("translate(0, {}) scale(1,-1)", height));

    for (p1, p2) in paths {
        item_group = item_group.add(
            svg::node::element::Line::new()
                .set("x1", p1.x)
                .set("y1", p1.y)
                .set("x2", p2.x)
                .set("y2", p2.y),
        );
    }

    svg_doc = svg_doc.add(item_group);

    svg::save("geom_perf.svg", &svg_doc).context("Failed to write svg")
}

const WIDTH: usize = 100;
const LENGTH: usize = 100;

const CELL_WIDTH: f64 = 2.0;
const CELL_LENGTH: f64 = 3.0;

fn generate_scene() -> Vec<Box<dyn Shape<WorldSpace>>> {
    let mut scene: Vec<Box<dyn Shape<WorldSpace>>> = Vec::new();

    for i in 0..WIDTH {
        for j in 0..LENGTH {
            let cell_x = i as f64 * CELL_WIDTH;
            let cell_z = j as f64 * CELL_LENGTH;
            let x1 = cell_x + 0.15;
            let x2 = cell_x + CELL_WIDTH - 0.15;

            let z1 = cell_z + 0.15;
            let z2 = cell_z + CELL_LENGTH - 0.15;
            scene.push(Box::new(RectPrism::new(
                WVec3::new(x1, 0.0, z1),
                WVec3::new(x2, 2.5, z2),
            )));
        }
    }

    scene
}
