//! SVG emission for strategy renders: outline and hatch strokes as separate
//! groups, ready for two-pen plotting.
use crate::scene::{RENDER_HEIGHT, RENDER_WIDTH};
use crate::strategies::{StrategyRender, Stroke};
use std::path::Path;

pub fn write(render: &StrategyRender, out_path: &Path) -> std::io::Result<()> {
    let doc = svg::Document::new()
        .set("width", format!("{}px", RENDER_WIDTH))
        .set("height", format!("{}px", RENDER_HEIGHT))
        .set("viewBox", (0, 0, RENDER_WIDTH, RENDER_HEIGHT))
        .set("fill", "none")
        .add(
            svg::node::element::Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", "100%")
                .set("height", "100%")
                .set("fill", "white"),
        )
        .add(stroke_group(&render.outline, 2.4))
        .add(stroke_group(&render.hatch, 1.4))
        .add(stroke_group(&render.contour, 1.0));
    svg::save(out_path, &doc)
}

fn stroke_group(segments: &[Stroke], width: f64) -> svg::node::element::Group {
    // The scene renders y-up; SVG is y-down.
    let mut group = svg::node::element::Group::new()
        .set(
            "transform",
            format!("translate(0, {}) scale(1,-1)", RENDER_HEIGHT),
        )
        .set("stroke", "black")
        .set("stroke-width", width)
        .set("stroke-linecap", "round");
    for (p1, p2) in segments {
        group = group.add(
            svg::node::element::Line::new()
                .set("x1", p1.x)
                .set("y1", p1.y)
                .set("x2", p2.x)
                .set("y2", p2.y),
        );
    }
    group
}
