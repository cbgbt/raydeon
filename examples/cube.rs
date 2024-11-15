use anyhow::{Context, Result};
use raydeon::shapes::RectPrism;
use raydeon::{Camera, Scene, WPoint3, WVec3};

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let scene = Scene::new(vec![Box::new(RectPrism::new(
        WVec3::new(-1.0, -1.0, -1.0),
        WVec3::new(1.0, 1.0, 1.0),
    ))]);

    let eye = WPoint3::new(4.0, 3.0, 2.0);
    let focus = WVec3::new(0.0, 0.0, 0.0);
    let up = WVec3::new(0.0, 0.0, 1.0);

    let fovy = 50.0;
    let width = 1024.0;
    let height = 1024.0;
    let znear = 0.1;
    let zfar = 10.0;

    let camera = Camera::look_at(eye, focus, up).perspective(fovy, width, height, znear, zfar);

    let paths = scene.attach_camera(camera).render();

    // We currently don't have any functionality to aid in emitting SVG images, so you will
    // be required to use the [svg crate.](https://crates.io/crates/svg)
    let mut svg_doc = svg::Document::new()
        .set("width", "8in")
        .set("height", "8in")
        .set("viewBox", (0, 0, width, height))
        .set("stroke-width", "0.7mm")
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

    for path in paths {
        let (p1, p2) = (path.p1, path.p2);
        item_group = item_group.add(
            svg::node::element::Line::new()
                .set("x1", p1.x)
                .set("y1", p1.y)
                .set("x2", p2.x)
                .set("y2", p2.y),
        );
    }

    svg_doc = svg_doc.add(item_group);

    svg::save("cube.svg", &svg_doc).context("Failed to write svg")
}
