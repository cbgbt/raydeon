use raydeon::lights::PointLight;
use raydeon::material::Material;
use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{Camera, Scene, SceneLighting, WPoint3, WVec3};
use std::sync::Arc;

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let scene = Scene::new()
        .with_geometry(vec![
            Arc::new(AxisAlignedCuboid::tagged(
                (-1.0, -1.0, -1.0),
                (1.0, 1.0, 1.0),
                Material::new(3.0, 2.0, 2.0, 0),
            )),
            Arc::new(AxisAlignedCuboid::tagged(
                (1.8, -1.0, -1.0),
                (3.8, 1.0, 1.0),
                Material::new(2.0, 2.0, 2.0, 0),
            )),
            Arc::new(AxisAlignedCuboid::tagged(
                (-1.4, 1.8, -1.0),
                (0.6, 3.8, 1.0),
                Material::new(3.0, 2.0, 2.0, 0),
            )),
        ])
        .with_lighting(
            SceneLighting::new()
                .with_lights(vec![Arc::new(PointLight::new(
                    20.0,
                    100.0,
                    (5.5, 12.0, 7.3),
                    0.0,
                    0.09,
                    0.23,
                ))])
                .with_ambient_lighting(0.13),
        );

    let eye = WPoint3::new(8.0, 6.0, 4.0);
    let focus = WVec3::new(0.0, 0.0, 0.0);
    let up = WVec3::new(0.0, 0.0, 1.0);

    let fovy = 50.0;
    let width = 1024;
    let height = 1024;
    let znear = 0.1;
    let zfar = 20.0;

    let camera = Camera::new()
        .look_at(eye, focus, up)
        .perspective(fovy, width, height, znear, zfar);

    let render_result = scene
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

    for path in render_result
        .geometry_paths
        .iter()
        .chain(render_result.hatch_paths.iter())
    {
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
    println!("{}", svg_doc);
}
