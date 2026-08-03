//! The contour engine: turning a material's [`ContourStyle`] into world-space
//! iso-lines over one shape's hatch surfaces.
//!
//! `style` states, and validates, WHICH contours a material draws and how
//! densely their fields are sampled; nothing there samples a field or walks
//! a grid. This module does that: per surface it builds a `SurfaceGrid`
//! (`grid`), samples each `ContourField` (`lines::surface_tone` for `Tone`,
//! `normal · view` for `Silhouette`), extracts the iso-level with marching
//! squares (`march`), and maps crossings back into lifted world-space
//! segments — planar surfaces clipped to their outline minus holes.

pub(crate) mod grid;
mod march;
mod style;

pub use style::{ContourField, ContourResolution, ContourStyle};

use super::surface::HatchSurface;
use crate::scene::SceneCamera;
use crate::{DrawableShape, LineSegment3D, WPoint3, WVec3, WorldSpace};
use grid::SurfaceGrid;
use march::{iso_polylines, EdgeCrossing, GridValues};

/// The world-space contour lines `style` calls for on `shape`, already
/// occluded by the same lift hatching uses but not yet clipped against the
/// scene.
///
/// Per surface, a `SurfaceGrid` is built at `style.resolution()`; per
/// `ContourField` on that surface, every node (and, lazily, every cell's
/// center — needed only to break a marching-squares saddle tie) is sampled,
/// the iso level is extracted, and each crossing is placed at its lifted
/// world position. Planar surfaces then clip the raw segments to the
/// outline minus its holes; other kinds need no clip.
pub(crate) fn contour_shape(
    cam: &SceneCamera,
    shape: &DrawableShape,
    style: &ContourStyle,
) -> Vec<LineSegment3D<WorldSpace>> {
    shape
        .hatch_surfaces()
        .iter()
        .flat_map(|surface| contour_surface(cam, shape, surface, style))
        .collect()
}

fn contour_surface(
    cam: &SceneCamera,
    shape: &DrawableShape,
    surface: &HatchSurface,
    style: &ContourStyle,
) -> Vec<LineSegment3D<WorldSpace>> {
    let grid = grid::surface_grid(surface, style.resolution());
    style
        .fields()
        .iter()
        .flat_map(|field| field_contours(cam, shape, surface, &grid, field))
        .collect()
}

/// The world-space segments one `ContourField`'s iso-line contributes.
fn field_contours(
    cam: &SceneCamera,
    shape: &DrawableShape,
    surface: &HatchSurface,
    grid: &SurfaceGrid,
    field: &ContourField,
) -> Vec<LineSegment3D<WorldSpace>> {
    let values = sample_grid(cam, shape, grid, field);
    let polylines = iso_polylines(&values, field_iso(field));

    let raw_segments: Vec<LineSegment3D<WorldSpace>> = polylines
        .into_iter()
        .flat_map(|crossings| {
            crossings
                .windows(2)
                .map(|pair| {
                    LineSegment3D::new_segment(
                        crossing_position(grid, &pair[0]),
                        crossing_position(grid, &pair[1]),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();

    match surface {
        HatchSurface::Planar(planar) => raw_segments
            .iter()
            .flat_map(|segment| clip_planar_segment(planar, segment))
            .collect(),
        HatchSurface::Sphere(_) | HatchSurface::Revolution(_) => raw_segments,
    }
}

/// The scalar `field` reads at a lifted point with the given outward
/// normal.
fn field_value(
    cam: &SceneCamera,
    shape: &DrawableShape,
    field: &ContourField,
    point: WPoint3,
    normal: WVec3,
) -> f64 {
    match field {
        ContourField::Tone(_) => {
            super::lines::surface_tone(cam.scene(), shape, cam.eye(), point, normal)
        }
        ContourField::Silhouette => normal.normalize().dot((cam.eye() - point).normalize()),
    }
}

/// The iso level `field`'s contour lies on: a `Tone` field's own threshold,
/// or the zero contour of `Silhouette`'s `normal · view`.
fn field_iso(field: &ContourField) -> f64 {
    match field {
        ContourField::Tone(threshold) => threshold.into_inner(),
        ContourField::Silhouette => 0.0,
    }
}

/// Samples `field` at every node of `grid`. A cell's center (the average of
/// its four corners' lifted positions and normals) is sampled lazily, only
/// if marching reaches a saddle cell that needs it to break its tie — never
/// eagerly for the whole grid.
fn sample_grid<'a>(
    cam: &'a SceneCamera,
    shape: &'a DrawableShape,
    grid: &'a SurfaceGrid,
    field: &'a ContourField,
) -> GridValues<'a> {
    let nodes: Vec<f64> = (0..grid.rows)
        .flat_map(|row| (0..grid.cols).map(move |col| (row, col)))
        .map(|(row, col)| {
            let (point, normal) = grid.node(row, col);
            field_value(cam, shape, field, point, normal)
        })
        .collect();

    GridValues::new(grid.rows, grid.cols, grid.wraps, nodes, move |row, col| {
        let (point, normal) = cell_center(grid, row, col);
        field_value(cam, shape, field, point, normal)
    })
}

/// The average lifted position and normal of a cell's four corners: an
/// approximate but real point on the surface, used only to sample the field
/// a second time when a saddle needs breaking.
fn cell_center(grid: &SurfaceGrid, row: usize, col: usize) -> (WPoint3, WVec3) {
    let (p00, n00) = grid.node(row, col);
    let (p01, n01) = grid.node(row, col + 1);
    let (p10, n10) = grid.node(row + 1, col);
    let (p11, n11) = grid.node(row + 1, col + 1);

    let sum = p00.to_vector() + p01.to_vector() + p10.to_vector() + p11.to_vector();
    let position = (sum / 4.0).to_point();
    let normal_sum = n00 + n01 + n10 + n11;
    let normal = if normal_sum.square_length() > 0.0 {
        normal_sum.normalize()
    } else {
        n00
    };
    (position, normal)
}

/// The world position of one crossing: linear interpolation, by its
/// fraction, between its edge's two lifted node positions.
fn crossing_position(grid: &SurfaceGrid, crossing: &EdgeCrossing) -> WPoint3 {
    let ((r0, c0), (r1, c1)) = crossing.edge.nodes();
    let (p0, _) = grid.node(r0, c0);
    let (p1, _) = grid.node(r1, c1);
    p0 + (p1 - p0) * crossing.fraction
}

/// Clips one raw contour segment to `surface`'s outline minus its holes,
/// using the same convex half-plane and hole-slab math `hatch::lines` clips
/// hatch lines with — just applied to an arbitrary segment direction
/// instead of a whole hatch pass.
fn clip_planar_segment(
    surface: &super::surface::PlanarSurface,
    segment: &LineSegment3D<WorldSpace>,
) -> Vec<LineSegment3D<WorldSpace>> {
    let lift = surface.normal() * super::lines::LINE_LIFT;
    let anchor = surface.to_face(segment.p1()).to_vector();
    let dir = surface.to_face(segment.p2()) - surface.to_face(segment.p1());

    let Some((lo, hi)) = super::lines::clip_to_outline(surface, anchor, dir, 0.0, 1.0) else {
        return Vec::new();
    };

    super::lines::subtract_holes(surface.holes(), anchor, dir, (lo.max(0.0), hi.min(1.0)))
        .into_iter()
        .filter(|(start, end)| end - start > 1.0e-9)
        .map(|(start, end)| {
            let p1 = surface.to_world((anchor + dir * start).to_point()) + lift;
            let p2 = surface.to_world((anchor + dir * end).to_point()) + lift;
            LineSegment3D::new_segment(p1, p2)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hatch::style::ToneThreshold;

    fn threshold(value: f64) -> ToneThreshold {
        ToneThreshold::try_new(value).expect("a tone fraction is a valid threshold")
    }

    use crate::hatch::surface::{FaceBox, FacePoint, PlanarSurface};
    use crate::lights::PointLight;
    use crate::shapes::{Quad, Sphere};
    use crate::{Camera, DrawableShape, PenId, Scene, SceneLighting, StrokeKind, ToneWhite};
    use std::sync::Arc;

    fn tone_white() -> ToneWhite {
        ToneWhite::try_new(1.0).expect("1.0 is a valid tone white")
    }

    fn floor(pen: usize, material_contours: Option<ContourStyle>) -> DrawableShape {
        DrawableShape::new()
            .geometry(Arc::new(
                Quad::new()
                    .origin(WPoint3::new(-4.0, -4.0, 0.0))
                    .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)])
                    .dims([8.0, 8.0])
                    .build(),
            ))
            .material(
                crate::Material::new()
                    .diffuse(1.0)
                    .pen(PenId::new(pen))
                    .maybe_contours(material_contours)
                    .build(),
            )
            .build()
    }

    fn overhead_camera() -> Camera {
        Camera::new()
            .observation(
                Camera::look_at(
                    WPoint3::new(0.0, 0.0, 12.0),
                    WVec3::new(0.0, 0.0, 0.0),
                    WVec3::new(0.0, 1.0, 0.0),
                )
                .expect("looks straight down at the floor"),
            )
            .perspective(
                Camera::perspective(50.0, 256, 256, 0.1, 40.0).expect("well-formed test frustum"),
            )
            .build()
    }

    #[test]
    fn a_scene_without_contours_draws_no_contour_strokes() {
        let scene = Scene::new()
            .geometry(vec![floor(0, None)])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((0.0, 0.0, 8.0))
                            .intensity(2.0)
                            .build(),
                    )])
                    .with_tone_white(tone_white()),
            )
            .build();

        let rendering = scene.attach_camera(overhead_camera()).render();
        assert!(
            rendering
                .strokes()
                .iter()
                .all(|stroke| stroke.kind != StrokeKind::Contour),
            "a material without `contours` must draw no Contour strokes"
        );
    }

    #[test]
    fn an_opted_in_material_draws_contour_strokes() {
        let style = ContourStyle::shadow(threshold(0.3));
        let scene = Scene::new()
            .geometry(vec![floor(0, Some(style))])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((3.0, 0.0, 2.0))
                            .intensity(1.0)
                            .build(),
                    )])
                    .with_tone_white(tone_white()),
            )
            .build();

        let rendering = scene.attach_camera(overhead_camera()).render();
        assert!(
            rendering
                .strokes()
                .iter()
                .any(|stroke| stroke.kind == StrokeKind::Contour),
            "an opted-in material must draw at least one Contour stroke"
        );
    }

    #[test]
    fn clip_planar_segment_keeps_only_the_parts_outside_the_hole() {
        let surface = PlanarSurface::try_new(
            WPoint3::zero(),
            [WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)],
            vec![
                FacePoint::new(0.0, 0.0),
                FacePoint::new(4.0, 0.0),
                FacePoint::new(4.0, 4.0),
                FacePoint::new(0.0, 4.0),
            ],
            vec![FaceBox::new(
                FacePoint::new(1.0, 1.0),
                FacePoint::new(3.0, 3.0),
            )],
        )
        .expect("a square with a hole is a surface");

        let lift = surface.normal() * super::super::lines::LINE_LIFT;
        let p1 = surface.to_world(FacePoint::new(0.0, 2.0)) + lift;
        let p2 = surface.to_world(FacePoint::new(4.0, 2.0)) + lift;
        let segment = LineSegment3D::new_segment(p1, p2);

        let pieces = clip_planar_segment(&surface, &segment);
        assert!(!pieces.is_empty(), "the segment survives outside the hole");
        for piece in pieces {
            let a = surface.to_face(piece.p1()).x;
            let b = surface.to_face(piece.p2()).x;
            assert!(
                (a <= 1.0 && b <= 1.0) || (a >= 3.0 && b >= 3.0),
                "a surviving piece [{a}, {b}] must lie entirely outside the hole's [1, 3]"
            );
        }
    }

    #[test]
    fn a_cast_shadow_scenes_tone_contour_lies_near_the_analytic_shadow_edge() {
        // A point light straight above and in front of a wall, casting a
        // hard shadow onto the floor behind it. Similar triangles give the
        // exact floor y-coordinate where the shadow's edge falls.
        let light_y = -2.0;
        let light_z = 10.0;
        let wall_y = 0.0;
        let wall_height = 3.0;
        let t_edge = light_z / (light_z - wall_height);
        let shadow_edge_y = light_y + t_edge * (wall_y - light_y);

        let wall = DrawableShape::new()
            .geometry(Arc::new(
                Quad::new()
                    .origin(WPoint3::new(-4.0, wall_y, 0.0))
                    .basis([WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 0.0, 1.0)])
                    .dims([8.0, wall_height])
                    .build(),
            ))
            .material(
                crate::Material::new()
                    .diffuse(1.0)
                    .pen(PenId::new(1))
                    .build(),
            )
            .build();

        let style = ContourStyle::shadow(threshold(0.05));
        let ground = floor(0, Some(style.clone()));

        let scene = Scene::new()
            .geometry(vec![ground.clone(), wall])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((0.0, light_y, light_z))
                            .intensity(3.0)
                            .build(),
                    )])
                    .with_tone_white(tone_white()),
            )
            .build();

        let cam = scene.attach_camera(overhead_camera());
        let segments = contour_shape(&cam, &ground, &style);
        assert!(
            !segments.is_empty(),
            "the shadow edge must draw a Tone contour"
        );

        // Only the crossing near the wall (small |x|) and away from the
        // floor's own outer edge (a ray exactly along a shape's own boundary
        // can miss self-recognition, an unrelated renderer edge case, not a
        // contour defect) — where the point light's shadow boundary is
        // closest to the straight-line analytic edge.
        let near_center: Vec<f64> = segments
            .iter()
            .flat_map(|segment| [segment.p1(), segment.p2()])
            .filter(|point| point.x.abs() < 0.5 && point.y.abs() < 3.5)
            .map(|point| point.y)
            .collect();
        assert!(
            !near_center.is_empty(),
            "test fixture sanity: some crossing near x=0"
        );
        for y in near_center {
            assert!(
                (y - shadow_edge_y).abs() < 1.0,
                "contour y {y} should sit near the analytic shadow edge {shadow_edge_y}"
            );
        }
    }

    #[test]
    fn a_sphere_silhouette_contour_matches_the_analytic_occluding_circle() {
        let sphere = Sphere::new()
            .center(WPoint3::new(0.0, 0.0, 0.0))
            .radius(2.0)
            .build();
        let drawable = DrawableShape::new()
            .geometry(Arc::new(sphere))
            .material(
                crate::Material::new()
                    .diffuse(1.0)
                    .pen(PenId::new(0))
                    .contours(ContourStyle::silhouette())
                    .build(),
            )
            .build();
        let scene = Scene::new()
            .geometry(vec![drawable.clone()])
            .lighting(
                SceneLighting::new()
                    .with_lights(vec![Arc::new(
                        PointLight::new()
                            .position((10.0, 0.0, 0.0))
                            .intensity(1.0)
                            .build(),
                    )])
                    .with_tone_white(tone_white()),
            )
            .build();

        let eye = WPoint3::new(0.0, -10.0, 0.0);
        let camera = Camera::new()
            .observation(
                Camera::look_at(eye, WVec3::zero(), WVec3::new(0.0, 0.0, 1.0))
                    .expect("looks at the sphere's center"),
            )
            .perspective(
                Camera::perspective(50.0, 256, 256, 0.1, 40.0).expect("well-formed test frustum"),
            )
            .build();
        let cam = scene.attach_camera(camera);

        let style = ContourStyle::silhouette();
        let segments = contour_shape(&cam, &drawable, &style);
        assert!(
            !segments.is_empty(),
            "the sphere must draw a silhouette contour"
        );

        let center = WPoint3::new(0.0, 0.0, 0.0);
        let to_eye = eye - center;
        let distance = to_eye.length();
        let look = to_eye / distance;
        let analytic_center = center + look * (2.0 * 2.0 / distance);
        let analytic_radius = 2.0 * (1.0 - (2.0 / distance).powi(2)).sqrt();

        for segment in &segments {
            for point in [segment.p1(), segment.p2()] {
                let offset = point - analytic_center;
                let radial = offset - look * offset.dot(look);
                let radius = radial.length();
                assert!(
                    (radius - analytic_radius).abs() < 0.5,
                    "contour radius {radius} vs analytic {analytic_radius}"
                );
            }
        }
    }
}
