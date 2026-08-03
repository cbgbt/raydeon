//! Renders the architectural test scene once per hatching strategy and
//! writes each result to `renders/<strategy>.svg` for visual comparison.
mod hatch;
mod scene;
mod shapes;
mod strategies;
mod svg_out;

use std::path::PathBuf;
use std::time::Instant;

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("renders");
    std::fs::create_dir_all(&out_dir)?;

    let test = scene::build();
    for (name, strategy) in strategies::all() {
        let started = Instant::now();
        let render = strategy(&test);
        let elapsed = started.elapsed();

        let out_path = out_dir.join(format!("{name}.svg"));
        svg_out::write(&render, &out_path)?;
        println!(
            "{name}: {} outline strokes, {} hatch strokes in {elapsed:.2?} -> {}",
            render.outline.len(),
            render.hatch.len(),
            out_path.display(),
        );
    }
    Ok(())
}
