use raydeon::lights::PointLight;
use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{Camera, Scene, SceneLighting, WPoint3, WVec3};
use raydeon::{DrawableShape, Material};
use std::sync::Arc;

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let cube_material = Material::new()
        .diffuse(3.0)
        .specular(2.0)
        .shininess(2.0)
        .build();
    let scene = Scene::new()
        .geometry(vec![
            DrawableShape::new()
                .geometry(Arc::new(
                    AxisAlignedCuboid::new()
                        .min((-1.0, -1.0, -1.0))
                        .max((1.0, 1.0, 1.0))
                        .build(),
                ))
                .material(cube_material)
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(
                    AxisAlignedCuboid::new()
                        .min((1.8, -1.0, -1.0))
                        .max((3.8, 1.0, 1.0))
                        .build(),
                ))
                .material(cube_material)
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(
                    AxisAlignedCuboid::new()
                        .min((-1.4, 1.8, -1.0))
                        .max((0.6, 3.8, 1.0))
                        .build(),
                ))
                .material(cube_material)
                .build(),
        ])
        .lighting(
            SceneLighting::new()
                .with_lights(vec![Arc::new(
                    PointLight::new()
                        .position((5.5, 12.0, 7.3))
                        .intensity(20.0)
                        .specular_intensity(100.0)
                        .constant_attenuation(0.0)
                        .linear_attenuation(0.09)
                        .quadratic_attenuation(0.23)
                        .build(),
                )])
                .with_ambient_lighting(0.13),
        )
        .build();

    let eye = WPoint3::new(8.0, 6.0, 4.0);
    let focus = WVec3::new(0.0, 0.0, 0.0);
    let up = WVec3::new(0.0, 0.0, 1.0);

    let fovy = 50.0;
    let width = 1024;
    let height = 1024;
    let znear = 0.1;
    let zfar = 20.0;

    let camera = Camera::new()
        .observation(
            Camera::look_at(eye, focus, up)
                .expect("the example's camera looks at a point in front of it"),
        )
        .perspective(
            Camera::perspective(fovy, width, height, znear, zfar)
                .expect("the example's frustum parameters are well formed"),
        )
        .build();

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

    for stroke in render_result.strokes() {
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
