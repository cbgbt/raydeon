## lnrs
A port of Michael Fogleman's [ln](https://github.com/fogleman/ln) in Rust.

Currently less featureful and probably also less performant.


## Example
Draws the cube which is included as an example in the original `ln` repo.

We currently don't have any functionality to aid in emitting SVG images, so you will
be required to use the [svg crate.](https://crates.io/crates/svg)

```rust
use lnrs::shapes::Cube;
use lnrs::{Camera, Scene, WPoint3, WVec3};

fn main() {
    let scene = Scene::new(vec![Box::new(Cube::new(
        WVec3::new(-1.0, -1.0, -1.0),
        WVec3::new(1.0, 1.0, 1.0)
    ))]);
    let camera = Camera::look_at(
            WPoint3::new(4.0, 3.0, 2.0),
            WVec3::new(0.0, 0.0, 0.0),
            WVec3::new(0.0, 0.0, 1.0),
        )
        .perspective(50.0, 1024.0, 1024.0, 0.1, 10.0);

    let paths = scene.render(camera, 0.1);

    let mut svg_doc = svg::Document::new()
        .set("width", "8in")
        .set("height", "8in")
        .set("viewBox", (0, 0, 1024, 1024))
        .set("stroke-width", "0.7mm")
        .set("stroke", "black")
        .set("fill", "none")
        .add(svg::node::element::Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", "white"));

    let mut item_group = svg::node::element::Group::new()
        .set("transform", format!("translate(0, {}) scale(1,-1)", 1024));
    
    for (p1, p2) in &paths.lines {
        item_group = item_group.add(svg::node::element::Line::new()
            .set("x1", p1.x)
            .set("y1", p1.y)
            .set("x2", p2.x)
            .set("y2", p2.y));
    }

    svg_doc = svg_doc.add(item_group);

    svg::save("out.svg", &svg_doc);
}
```