//! Minimal reproduction probe: does `clip_and_project` drop world segments
//! hidden behind a cuboid, with the storefront's camera geometry?
use raydeon::shapes::{AxisAlignedCuboid, Quad};
use raydeon::{Camera, DrawableShape, LineSegment3D, Material, Scene, WPoint3, WVec3};
use std::sync::Arc;

fn main() {
    let ground = Quad::new()
        .origin(WPoint3::new(-9.5, -7.5, 0.0))
        .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)])
        .dims([17.5, 10.7])
        .build();
    let planter = AxisAlignedCuboid::new()
        .min((-4.6, -1.6, 0.0))
        .max((-3.9, -0.9, 0.55))
        .build();

    let scene = Scene::new()
        .geometry(vec![
            DrawableShape::new()
                .geometry(Arc::new(ground) as Arc<dyn raydeon::Shape>)
                .material(Material::new().diffuse(1.0).build())
                .build(),
            DrawableShape::new()
                .geometry(Arc::new(planter) as Arc<dyn raydeon::Shape>)
                .material(Material::new().diffuse(1.0).build())
                .build(),
        ])
        .build();

    let camera = Camera::new()
        .observation(
            Camera::look_at(
                WPoint3::new(-7.8, -10.5, 3.9),
                WVec3::new(0.6, 0.2, 2.2),
                WVec3::new(0.0, 0.0, 1.0),
            )
            .expect("probe camera is valid"),
        )
        .perspective(Camera::perspective(41.0, 1024, 768, 0.1, 60.0).expect("probe frustum"))
        .build();
    let sc = scene.attach_camera(camera);

    // Ground-level probe lines running left-to-right past the planter,
    // lifted like hatch lines. Spans hidden by the box must vanish.
    for y in [-0.5, -1.0, -1.25, -2.0] {
        let probe =
            LineSegment3D::new_segment(WPoint3::new(-6.0, y, 0.006), WPoint3::new(-1.5, y, 0.006));
        let out = sc.clip_and_project(&[probe]);
        println!("probe y={y}: {} surviving segments", out.len());
        for seg in &out {
            println!(
                "   screen ({:7.1},{:6.1}) -> ({:7.1},{:6.1})",
                seg.p1.x, seg.p1.y, seg.p2.x, seg.p2.y
            );
        }
    }

    // Where does the planter project? (corners of its front face)
    let t = sc.clip_and_project(&[
        LineSegment3D::new_segment(WPoint3::new(-4.6, -1.6, 0.0), WPoint3::new(-3.9, -1.6, 0.0)),
        LineSegment3D::new_segment(
            WPoint3::new(-4.6, -1.6, 0.55),
            WPoint3::new(-3.9, -1.6, 0.55),
        ),
    ]);
    for seg in &t {
        println!(
            "box front edge: ({:7.1},{:6.1}) -> ({:7.1},{:6.1})",
            seg.p1.x, seg.p1.y, seg.p2.x, seg.p2.y
        );
    }
}
