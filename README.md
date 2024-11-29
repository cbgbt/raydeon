## raydeon

A port of Michael Fogleman's [ln](https://github.com/fogleman/ln) in Rust, with
exposed bindings for Python. Like Michael, I also developed this to plot images
with a [pen plotter](https://shop.evilmadscientist.com/productsmenu/846). To
steal his words (because this port isn't theft enough):

> The output of an OpenGL pipeline is a rastered image. The output of raydeon is
> a set of 2D vector paths.

This repository has added support for screen-space hatching based on lights
placed within the scene.

![](/raydeon/examples/cityscape.png)

## Example

Have a look at any of the [Rust examples](/raydeon/examples) or
[Python examples](/pyraydeon/examples/).

The following Rust code draws the 3 cubes below with hatching based on the
lighting.

```rust
use raydeon::lights::PointLight;
use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{Camera, Scene, SceneLighting, WPoint3, WVec3};
use raydeon::{DrawableShape, Material};
use std::sync::Arc;

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let cube_material = Material::new_mat(3.0, 2.0, 2.0, 0);
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
                .with_lights(vec![Arc::new(PointLight::new(
                    20.0,
                    100.0,
                    (5.5, 12.0, 7.3),
                    0.0,
                    0.09,
                    0.23,
                ))])
                .with_ambient_lighting(0.13),
        )
        .construct();

    let eye = WPoint3::new(8.0, 6.0, 4.0);
    let focus = WVec3::new(0.0, 0.0, 0.0);
    let up = WVec3::new(0.0, 0.0, 1.0);

    let fovy = 50.0;
    let width = 1024;
    let height = 1024;
    let znear = 0.1;
    let zfar = 20.0;

    let camera = Camera::configure()
        .observation(Camera::look_at(eye, focus, up))
        .perspective(Camera::perspective(fovy, width, height, znear, zfar))
        .build();

    let render_result = scene.attach_camera(camera).render_with_lighting();

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

    for path in render_result {
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
```

![](/raydeon/examples/lit_cubes_expected.svg)
