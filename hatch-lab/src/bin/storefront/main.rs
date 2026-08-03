//! A shopfront art piece exercising the hatching engine: a composed facade
//! with true openings, a recessed display alcove lit by its own lamp, a
//! striped awning, and four pens rendered as colored plot layers.
use euclid::Angle;
use raydeon::lights::PointLight;
use raydeon::shapes::{AxisAlignedCuboid, Quad, Sphere, Triangle};
use raydeon::{
    Camera, DrawableShape, HatchSpacing, HatchStyle, Material, PenId, Rendering, Scene,
    SceneLighting, Shape, StrokeKind, TonalPass, ToneThreshold, ToneWhite, WPoint3, WVec3,
};
use std::path::PathBuf;
use std::sync::Arc;

mod detail;
use detail::Awning;

const WIDTH: usize = 1024;
const HEIGHT: usize = 768;

/// Afternoon sun, low and to the left: long shadows across the street.
const SUN: (f64, f64, f64) = (-14.0, -9.0, 10.0);
/// A warm lamp inside the display alcove.
const LAMP: (f64, f64, f64) = (1.5, 0.35, 2.9);

/// The four physical pens of the plot, with preview colors.
const PENS: [(usize, &str, &str); 4] = [
    (0, "ink", "#22242a"),
    (1, "walnut", "#6d4326"),
    (2, "leaf", "#43683b"),
    (3, "slate", "#41597a"),
];

fn main() -> std::io::Result<()> {
    let scene = build_scene();
    let camera = Camera::new()
        .observation(
            Camera::look_at(
                WPoint3::new(-7.8, -10.5, 3.9),
                WVec3::new(0.6, 0.2, 2.2),
                WVec3::new(0.0, 0.0, 1.0),
            )
            .expect("the storefront camera looks at the shop"),
        )
        .perspective(
            Camera::perspective(41.0, WIDTH, HEIGHT, 0.1, 60.0)
                .expect("the storefront frustum is well formed"),
        )
        .build();

    let rendering = scene.attach_camera(camera).render();
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("renders/storefront.svg");
    write_svg(&rendering, &out)?;
    println!(
        "storefront: {} strokes across {} pens -> {}",
        rendering.strokes().len(),
        rendering.pens().len(),
        out.display()
    );
    Ok(())
}

fn build_scene() -> Scene {
    let base = HatchSpacing::try_new(0.22).expect("base spacing is positive");
    let tight = HatchSpacing::try_new(0.15).expect("tight spacing is positive");
    let sun = WPoint3::new(SUN.0, SUN.1, SUN.2);
    let lamp = WPoint3::new(LAMP.0, LAMP.1, LAMP.2);

    let material = |style: HatchStyle, pen: usize| {
        Material::new()
            .diffuse(1.0)
            .specular(0.3)
            .shininess(8.0)
            .pen(PenId::new(pen))
            .hatch(style)
            .build()
    };
    let ink_tonal = material(HatchStyle::tonal_crosshatch(base), 0);
    let ink_stochastic = material(HatchStyle::stochastic(base), 0);
    let wood_grain = HatchStyle::Tonal {
        passes: vec![
            TonalPass {
                angle: Angle::degrees(90.0),
                spacing: tight,
                threshold: ToneThreshold::try_new(0.96).expect("grain threshold in range"),
            },
            TonalPass {
                angle: Angle::degrees(45.0),
                spacing: tight,
                threshold: ToneThreshold::try_new(0.35).expect("shade threshold in range"),
            },
        ],
    };
    let walnut_wood = Material::new()
        .diffuse(0.55)
        .specular(0.2)
        .shininess(6.0)
        .pen(PenId::new(1))
        .hatch(wood_grain)
        .build();
    let leaf_flow = material(HatchStyle::light_flow(sun, base), 2);
    let slate_tonal = material(HatchStyle::tonal_crosshatch(tight), 3);
    let pottery_flow = material(HatchStyle::light_flow(lamp, tight), 1);
    let ink_plain = Material::new().diffuse(1.0).build();
    let walnut_plain = Material::new().diffuse(1.0).pen(PenId::new(1)).build();

    let mut shapes: Vec<DrawableShape> = Vec::new();
    let mut add = |shape: Arc<dyn Shape>, mat: &Material| {
        shapes.push(
            DrawableShape::new()
                .geometry(shape)
                .material(mat.clone())
                .build(),
        );
    };

    // Street.
    add(
        Arc::new(quad(
            (-9.5, -7.5, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            [17.5, 10.7],
        )),
        &ink_stochastic,
    );

    // Facade at y = 0, composed around two true openings:
    // door x[-3.6,-2.2] z[0,2.6], display window x[-0.9,3.9] z[0.7,3.1].
    let facade = [
        ((-5.0, 0.0, 0.0), [1.4, 4.6]), // left of door
        ((-3.6, 0.0, 2.6), [1.4, 2.0]), // above door
        ((-2.2, 0.0, 0.0), [1.3, 4.6]), // pier between door and window
        ((-0.9, 0.0, 0.0), [4.8, 0.7]), // kneewall below window
        ((-0.9, 0.0, 3.1), [4.8, 1.5]), // above window
        ((3.9, 0.0, 0.0), [1.1, 4.6]),  // right of window
    ];
    for (origin, dims) in facade {
        add(
            Arc::new(quad(origin, (1.0, 0.0, 0.0), (0.0, 0.0, 1.0), dims)),
            &ink_tonal,
        );
    }
    // Building side seen from the left, and the parapet cap.
    add(
        Arc::new(quad(
            (-5.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0),
            [4.6, 3.0],
        )),
        &ink_tonal,
    );
    add(
        Arc::new(cuboid((-5.2, -0.15, 4.6), (5.2, 0.4, 4.9))),
        &ink_tonal,
    );

    // Display alcove behind the window, recessed 1.5 into the shop.
    add(
        Arc::new(quad(
            (-0.9, 0.0, 0.7),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            [4.8, 1.5],
        )),
        &ink_tonal,
    );
    add(
        Arc::new(quad(
            (-0.9, 1.5, 0.7),
            (1.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            [4.8, 2.4],
        )),
        &ink_tonal,
    );
    add(
        Arc::new(quad(
            (-0.9, 0.0, 3.1),
            (0.0, 1.0, 0.0),
            (1.0, 0.0, 0.0),
            [1.5, 4.8],
        )),
        &ink_tonal,
    );
    add(
        Arc::new(quad(
            (-0.9, 0.0, 0.7),
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0),
            [1.5, 2.4],
        )),
        &ink_tonal,
    );
    add(
        Arc::new(quad(
            (3.9, 0.0, 0.7),
            (0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0),
            [2.4, 1.5],
        )),
        &ink_tonal,
    );

    // The display: a thrown vase and bowl (ring contours), one glazed
    // sphere shaded toward the lamp, and a stack of books with a bauble.
    add(
        Arc::new(detail::vase(WPoint3::new(0.15, 0.85, 0.7))),
        &walnut_plain,
    );
    add(
        Arc::new(detail::bowl(WPoint3::new(1.5, 0.7, 0.7))),
        &walnut_plain,
    );
    add(Arc::new(sphere((2.9, 0.85, 1.32), 0.62)), &pottery_flow);
    add(
        Arc::new(cuboid((1.95, 0.95, 0.7), (2.42, 1.28, 0.79))),
        &walnut_wood,
    );
    add(
        Arc::new(cuboid((2.02, 1.0, 0.79), (2.36, 1.22, 0.86))),
        &walnut_wood,
    );
    add(Arc::new(sphere((2.19, 1.11, 0.97), 0.11)), &walnut_plain);

    // Door, recessed with reveals; sill under the display window.
    add(
        Arc::new(quad(
            (-3.6, 0.25, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            [1.4, 2.6],
        )),
        &walnut_wood,
    );
    add(
        Arc::new(quad(
            (-3.6, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 1.0),
            [0.25, 2.6],
        )),
        &ink_tonal,
    );
    add(
        Arc::new(quad(
            (-2.2, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0),
            [2.6, 0.25],
        )),
        &ink_tonal,
    );
    add(
        Arc::new(cuboid((-1.05, -0.12, 0.58), (4.05, 0.05, 0.7))),
        &walnut_plain,
    );
    add(Arc::new(sphere((-2.45, 0.2, 1.25), 0.09)), &walnut_plain);

    // Awning over the display window, striped in slate.
    let awning = Awning::new(
        WPoint3::new(-1.1, 0.0, 3.35),
        WVec3::new(0.0, -1.3, -0.6),
        5.2,
        0.4,
    );
    add(Arc::new(awning), &slate_tonal);
    for x in [-1.1, 4.1] {
        add(
            Arc::new(
                Triangle::new()
                    .v0((x, 0.0, 3.35))
                    .v1((x, -1.3, 2.75))
                    .v2((x, 0.0, 2.75))
                    .build(),
            ),
            &slate_tonal,
        );
    }

    // Hanging blade sign above the door.
    add(
        Arc::new(cuboid((-2.9, -0.9, 3.86), (-2.82, 0.0, 3.94))),
        &walnut_wood,
    );
    add(
        Arc::new(cuboid((-2.9, -0.85, 3.05), (-2.82, -0.15, 3.8))),
        &walnut_wood,
    );

    // Potted plants flanking the shopfront, and a crate by the window.
    for (pmin, pmax, center) in [
        ((-4.6, -1.6, 0.0), (-3.9, -0.9, 0.55), (-4.25, -1.25, 1.0)),
        ((4.05, -1.5, 0.0), (4.75, -0.8, 0.55), (4.4, -1.15, 1.0)),
    ] {
        add(Arc::new(cuboid(pmin, pmax)), &walnut_wood);
        add(Arc::new(sphere(center, 0.55)), &leaf_flow);
    }
    add(
        Arc::new(cuboid((2.2, -2.0, 0.0), (3.0, -1.2, 0.5))),
        &walnut_wood,
    );
    add(
        Arc::new(cuboid((2.35, -1.85, 0.5), (2.95, -1.35, 0.9))),
        &walnut_wood,
    );

    add(Arc::new(detail::pavement()), &ink_plain);
    add(Arc::new(detail::cracks()), &ink_plain);
    add(Arc::new(detail::brick_patches()), &ink_plain);

    Scene::new()
        .geometry(shapes)
        .lighting(
            SceneLighting::new()
                .with_lights(vec![
                    Arc::new(
                        PointLight::new()
                            .position(SUN)
                            .intensity(3.0)
                            .specular_intensity(0.5)
                            .constant_attenuation(1.0)
                            .linear_attenuation(0.012)
                            .quadratic_attenuation(0.0008)
                            .build(),
                    ),
                    Arc::new(
                        PointLight::new()
                            .position(LAMP)
                            .intensity(1.7)
                            .constant_attenuation(1.0)
                            .linear_attenuation(0.1)
                            .quadratic_attenuation(0.18)
                            .build(),
                    ),
                ])
                .with_ambient_lighting(0.16)
                .with_tone_white(ToneWhite::try_new(1.3).expect("tone white is positive")),
        )
        .build()
}

fn quad(origin: (f64, f64, f64), b0: (f64, f64, f64), b1: (f64, f64, f64), dims: [f64; 2]) -> Quad {
    Quad::new()
        .origin(WPoint3::new(origin.0, origin.1, origin.2))
        .basis([WVec3::new(b0.0, b0.1, b0.2), WVec3::new(b1.0, b1.1, b1.2)])
        .dims(dims)
        .build()
}

fn cuboid(min: (f64, f64, f64), max: (f64, f64, f64)) -> AxisAlignedCuboid {
    AxisAlignedCuboid::new().min(min).max(max).build()
}

fn sphere(center: (f64, f64, f64), radius: f64) -> Sphere {
    Sphere::new().center(center).radius(radius).build()
}

fn write_svg(rendering: &Rendering, out: &PathBuf) -> std::io::Result<()> {
    let mut doc = svg::Document::new()
        .set("width", format!("{WIDTH}px"))
        .set("height", format!("{HEIGHT}px"))
        .set("viewBox", (0, 0, WIDTH, HEIGHT))
        .set("fill", "none")
        .add(
            svg::node::element::Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", "100%")
                .set("height", "100%")
                .set("fill", "#fbfaf6"),
        );

    for (pen, name, color) in PENS {
        let pen = PenId::new(pen);
        let mut group = svg::node::element::Group::new()
            .set("id", format!("{}-{}", pen.value() + 1, name))
            .set("transform", format!("translate(0, {HEIGHT}) scale(1,-1)"))
            .set("stroke", color)
            .set("stroke-linecap", "round");
        let mut outlines = svg::node::element::Group::new().set("stroke-width", 2.3);
        let mut hatches = svg::node::element::Group::new().set("stroke-width", 1.35);
        let mut contours = svg::node::element::Group::new().set("stroke-width", 1.0);
        for stroke in rendering.strokes_for_pen(pen) {
            let line = svg::node::element::Line::new()
                .set("x1", stroke.p1.x)
                .set("y1", stroke.p1.y)
                .set("x2", stroke.p2.x)
                .set("y2", stroke.p2.y);
            match stroke.kind {
                StrokeKind::Outline => outlines = outlines.add(line),
                StrokeKind::Hatch => hatches = hatches.add(line),
                StrokeKind::Contour => contours = contours.add(line),
            }
        }
        group = group.add(outlines).add(hatches).add(contours);
        doc = doc.add(group);
    }
    svg::save(out, &doc)
}
