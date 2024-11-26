## raydeon

A port of Michael Fogleman's [ln](https://github.com/fogleman/ln) in Rust, with
exposed bindings for Python. Like Michael, I also developed this to plot images
with a [pen plotter](https://shop.evilmadscientist.com/productsmenu/846). To
steal his words (because this port isn't theft enough):

> The output of an OpenGL pipeline is a rastered image. The output of raydeon is
> a set of 2D vector paths.

![](/raydeon/examples/cityscape.png)

Currently less featureful though probably more performant than the prior art.

## Example

Have a look at any of the [Rust examples](/raydeon/examples) or
[Python examples](/pyraydeon/examples/).

The following Rust code draws the cube which is included as an example in the
original `ln` repo.

```rust
use raydeon::shapes::AxisAlignedCuboid;
use raydeon::{Camera, Scene, WPoint3, WVec3};
use std::sync::Arc;

fn main() {
    env_logger::Builder::from_default_env()
        .format_timestamp_nanos()
        .init();

    let scene = Scene::new().with_geometry(vec![Arc::new(AxisAlignedCuboid::new(
        WVec3::new(-1.0, -1.0, -1.0),
        WVec3::new(1.0, 1.0, 1.0),
    ))]);

    let eye = WPoint3::new(4.0, 3.0, 2.0);
    let focus = WVec3::new(0.0, 0.0, 0.0);
    let up = WVec3::new(0.0, 0.0, 1.0);

    let fovy = 50.0;
    let width = 1024;
    let height = 1024;
    let znear = 0.1;
    let zfar = 10.0;

    let camera = Camera::new()
        .look_at(eye, focus, up)
        .perspective(fovy, width, height, znear, zfar);

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
    println!("{}", svg_doc);
}
```

![](/raydeon/examples/cube.png)
