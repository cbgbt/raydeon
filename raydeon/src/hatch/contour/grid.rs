//! Turning a hatch surface into the lattice `march` walks: one parameterized
//! constructor per surface kind, each producing a row-major grid of LIFTED
//! world positions and outward normals — the position a `Tone` sample or an
//! emitted contour actually occupies (invariant 10), not the bare surface
//! point.

use super::ContourResolution;
use crate::hatch::lines::LINE_LIFT;
use crate::hatch::surface::{
    FaceBox, FacePoint, HatchSurface, PlanarSurface, ProfilePoint, RevolutionSurface, SphereSurface,
};
use crate::{WPoint3, WVec3};
use euclid::Angle;

/// Below this a surface's own extent is indistinguishable from nothing, and
/// a grid over it would be meaningless.
const MIN_EXTENT: f64 = 1.0e-9;

/// A lattice of `rows` by `cols` nodes laid over a hatch surface, row-major.
///
/// `wraps` marks a grid whose column axis is a closed loop (a sphere's or a
/// revolution's longitude): node `(row, cols)` is the same physical point as
/// node `(row, 0)`. Rows never wrap.
pub(crate) struct SurfaceGrid {
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) wraps: bool,
    nodes: Vec<(WPoint3, WVec3)>,
}

impl SurfaceGrid {
    fn new(rows: usize, cols: usize, wraps: bool, nodes: Vec<(WPoint3, WVec3)>) -> Self {
        debug_assert_eq!(nodes.len(), rows * cols, "nodes must fill the grid exactly");
        Self {
            rows,
            cols,
            wraps,
            nodes,
        }
    }

    /// The lifted position and outward normal at node `(row, col)`. `col` is
    /// taken modulo `cols`, so a caller can address the wrap seam's node
    /// `cols` without special-casing it.
    pub(crate) fn node(&self, row: usize, col: usize) -> (WPoint3, WVec3) {
        self.nodes[row * self.cols + (col % self.cols)]
    }

    /// How many cell columns this grid has: `cols` if it wraps (the last
    /// cell closes the seam back to column 0), else `cols - 1`.
    pub(crate) fn cell_cols(&self) -> usize {
        if self.wraps {
            self.cols
        } else {
            self.cols - 1
        }
    }
}

/// The grid `style.resolution()` calls for over `surface`, in whichever
/// parameterization its kind uses.
pub(crate) fn surface_grid(surface: &HatchSurface, resolution: ContourResolution) -> SurfaceGrid {
    let resolution = resolution.into_inner();
    match surface {
        HatchSurface::Planar(planar) => planar_grid(planar, resolution),
        HatchSurface::Sphere(sphere) => sphere_grid(sphere, resolution),
        HatchSurface::Revolution(revolution) => revolution_grid(revolution, resolution),
    }
}

/// A lattice over the outline's face-frame bounding box, anchored at the
/// box's minimum corner; nodes may fall past the outline itself — the
/// contour engine clips finished segments to the outline minus its holes
/// afterward, rather than masking the grid.
fn planar_grid(surface: &PlanarSurface, resolution: f64) -> SurfaceGrid {
    let bbox = outline_bbox(surface.outline());
    let extent_x = (bbox.max.x - bbox.min.x).max(MIN_EXTENT);
    let extent_y = (bbox.max.y - bbox.min.y).max(MIN_EXTENT);
    let steps_x = ((extent_x / resolution).ceil() as usize).max(1);
    let steps_y = ((extent_y / resolution).ceil() as usize).max(1);
    let step_x = extent_x / steps_x as f64;
    let step_y = extent_y / steps_y as f64;
    let (rows, cols) = (steps_y + 1, steps_x + 1);

    let normal = surface.normal();
    let lift = normal * LINE_LIFT;
    let nodes = (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .map(|(row, col)| {
            let face_point = FacePoint::new(
                bbox.min.x + col as f64 * step_x,
                bbox.min.y + row as f64 * step_y,
            );
            (surface.to_world(face_point) + lift, normal)
        })
        .collect();

    SurfaceGrid::new(rows, cols, false, nodes)
}

/// The bounding box, in face space, of an outline's corners.
fn outline_bbox(outline: &[FacePoint]) -> FaceBox {
    outline.iter().fold(
        FaceBox::new(
            FacePoint::new(f64::MAX, f64::MAX),
            FacePoint::new(f64::MIN, f64::MIN),
        ),
        |acc, corner| {
            FaceBox::new(
                FacePoint::new(acc.min.x.min(corner.x), acc.min.y.min(corner.y)),
                FacePoint::new(acc.max.x.max(corner.x), acc.max.y.max(corner.y)),
            )
        },
    )
}

/// Polar `(θ, φ)` about world Z. Pole rows are kept as degenerate rows (all
/// columns coincide at the same point) rather than special-cased away, so
/// the grid stays a plain rectangular lattice; `φ` wraps.
fn sphere_grid(surface: &SphereSurface, resolution: f64) -> SurfaceGrid {
    let radius = surface.radius();
    let theta_steps = ((std::f64::consts::PI * radius / resolution).ceil() as usize).max(1);
    let cols = ((std::f64::consts::TAU * radius / resolution).ceil() as usize).max(3);
    let rows = theta_steps + 1;

    let lifted_radius = radius + LINE_LIFT;
    let nodes = (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .map(|(row, col)| {
            let theta = row as f64 / theta_steps as f64 * std::f64::consts::PI;
            let phi = col as f64 / cols as f64 * std::f64::consts::TAU;
            let direction = WVec3::new(
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            );
            (surface.center() + direction * lifted_radius, direction)
        })
        .collect();

    SurfaceGrid::new(rows, cols, true, nodes)
}

/// Profile-arc-length by angle. Rows follow the profile at roughly
/// `resolution` spacing but always land exactly on every profile vertex, so
/// a kink is never smoothed away by interpolation; `φ` wraps.
fn revolution_grid(surface: &RevolutionSurface, resolution: f64) -> SurfaceGrid {
    let row_ts = revolution_row_ts(surface, resolution);
    let rows = row_ts.len();
    let cols = ((std::f64::consts::TAU * surface.max_radius() / resolution).ceil() as usize).max(3);

    let nodes = (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .map(|(row, col)| {
            let t = row_ts[row];
            let angle = Angle::radians(col as f64 / cols as f64 * std::f64::consts::TAU);
            let normal = surface.normal_at(t, angle);
            (surface.point_at(t, angle) + normal * LINE_LIFT, normal)
        })
        .collect();

    SurfaceGrid::new(rows, cols, true, nodes)
}

/// The profile arc-length of every row this grid samples at: every profile
/// vertex's own arc length, plus enough evenly-spaced samples within each
/// segment to keep consecutive rows roughly `resolution` apart.
fn revolution_row_ts(surface: &RevolutionSurface, resolution: f64) -> Vec<f64> {
    let mut ts = vec![0.0];
    let mut cursor = 0.0;
    for pair in surface.profile().windows(2) {
        let seg_len = profile_segment_length(pair[0], pair[1]);
        let steps = ((seg_len / resolution).ceil() as usize).max(1);
        for step in 1..=steps {
            ts.push(cursor + seg_len * step as f64 / steps as f64);
        }
        cursor += seg_len;
    }
    ts
}

fn profile_segment_length(a: ProfilePoint, b: ProfilePoint) -> f64 {
    let dr = b.radius - a.radius;
    let dh = b.height - a.height;
    (dr * dr + dh * dh).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hatch::surface::FaceBox as FB;
    use crate::WPoint3;

    fn unit_square() -> PlanarSurface {
        PlanarSurface::try_new(
            WPoint3::new(0.0, 0.0, 0.0),
            [WVec3::new(1.0, 0.0, 0.0), WVec3::new(0.0, 1.0, 0.0)],
            vec![
                FacePoint::new(0.0, 0.0),
                FacePoint::new(4.0, 0.0),
                FacePoint::new(4.0, 4.0),
                FacePoint::new(0.0, 4.0),
            ],
            vec![],
        )
        .expect("a square is a surface")
    }

    fn resolution(value: f64) -> ContourResolution {
        ContourResolution::try_new(value).expect("test resolutions are positive")
    }

    #[test]
    fn a_planar_grid_anchors_at_the_outlines_bbox_minimum() {
        let surface = unit_square();
        let grid = planar_grid(&surface, 1.0);
        let (node, normal) = grid.node(0, 0);
        assert_eq!(
            node,
            surface.to_world(FacePoint::new(0.0, 0.0)) + normal * LINE_LIFT
        );
        assert_eq!(normal, surface.normal());
        assert!(!grid.wraps);
    }

    #[test]
    fn a_planar_grid_does_not_wrap_and_never_wraps_columns() {
        let surface = unit_square();
        let grid = planar_grid(&surface, 1.0);
        assert_eq!(grid.rows, 5);
        assert_eq!(grid.cols, 5);
        assert_eq!(grid.cell_cols(), 4);
    }

    #[test]
    fn a_sphere_grid_lifts_every_node_off_the_sphere_and_wraps_columns() {
        let surface = SphereSurface::try_new(WPoint3::new(1.0, 2.0, 3.0), 2.0)
            .expect("a real sphere is a surface");
        let grid = sphere_grid(&surface, resolution(0.3).into_inner());

        assert!(grid.wraps);
        for row in 0..grid.rows {
            for col in 0..grid.cols {
                let (node, normal) = grid.node(row, col);
                let offset = (node - surface.center()).length();
                assert!(
                    (offset - (surface.radius() + LINE_LIFT)).abs() < 1.0e-9,
                    "node sits at {offset} from the center"
                );
                assert!((normal.length() - 1.0).abs() < 1.0e-9);
            }
        }
    }

    #[test]
    fn a_sphere_grids_pole_rows_are_degenerate() {
        let surface = SphereSurface::try_new(WPoint3::zero(), 1.0).expect("a real sphere");
        let grid = sphere_grid(&surface, resolution(0.3).into_inner());
        let (north_a, _) = grid.node(0, 0);
        let (north_b, _) = grid.node(0, 3);
        assert_eq!(
            north_a, north_b,
            "every column of the pole row is one point"
        );
    }

    #[test]
    fn a_revolution_grid_wraps_columns_and_includes_every_profile_vertex() {
        let profile = vec![
            ProfilePoint {
                radius: 1.0,
                height: 0.0,
            },
            ProfilePoint {
                radius: 1.0,
                height: 1.0,
            },
            ProfilePoint {
                radius: 1.6,
                height: 2.0,
            },
        ];
        let surface =
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 1.0), profile.clone())
                .expect("a kinked profile is a surface");

        let row_ts = revolution_row_ts(&surface, 0.3);
        let vertex_arc_lengths = [
            0.0,
            profile_segment_length(profile[0], profile[1]),
            profile_segment_length(profile[0], profile[1])
                + profile_segment_length(profile[1], profile[2]),
        ];
        for expected in vertex_arc_lengths {
            assert!(
                row_ts.iter().any(|&t| (t - expected).abs() < 1.0e-9),
                "row_ts {row_ts:?} must include the profile vertex at {expected}"
            );
        }

        let grid = revolution_grid(&surface, 0.3);
        assert!(grid.wraps);
    }

    #[test]
    fn a_revolution_grids_contour_terminates_at_the_foot_or_lip_row_never_wrapping_through_it() {
        // A cylinder's grid: `wraps` is set for the angle (column) axis, but
        // rows (the profile, foot to lip) never wrap. A synthetic field
        // shaped like an island touching only the lip (row 0) — inside for
        // rows 0-1 across a contiguous band of columns, outside everywhere
        // else — has no "row above 0" to close it off, so its contour must
        // be ONE OPEN chain whose two endpoints sit on the row-0 boundary;
        // it must never spuriously wrap all the way around the columns to
        // close on itself.
        let surface = RevolutionSurface::try_new(
            WPoint3::zero(),
            WVec3::new(0.0, 0.0, 1.0),
            vec![
                ProfilePoint {
                    radius: 1.0,
                    height: 0.0,
                },
                ProfilePoint {
                    radius: 1.0,
                    height: 2.0,
                },
            ],
        )
        .expect("a cylinder is a surface");
        let grid = revolution_grid(&surface, 0.3);
        assert!(grid.wraps, "the angle axis wraps");
        assert!(
            grid.rows >= 4,
            "test fixture sanity: enough rows to have an interior"
        );

        let inside_band = 2..5; // columns 2, 3, 4 are inside; the rest are not
        let value_at = |row: usize, col: usize| -> f64 {
            if row <= 1 && inside_band.contains(&col) {
                -1.0
            } else {
                1.0
            }
        };
        let nodes: Vec<f64> = (0..grid.rows)
            .flat_map(|row| (0..grid.cols).map(move |col| (row, col)))
            .map(|(row, col)| value_at(row, col))
            .collect();
        let centers: Vec<f64> = (0..grid.rows - 1)
            .flat_map(|row| (0..grid.cell_cols()).map(move |col| (row, col)))
            .map(|(row, col)| {
                // The average of the cell's four corners, matching how the
                // contour engine itself samples a saddle's center.
                let corners = [
                    value_at(row, col),
                    value_at(row, col + 1),
                    value_at(row + 1, col),
                    value_at(row + 1, col + 1),
                ];
                corners.iter().sum::<f64>() / 4.0
            })
            .collect();

        let values = crate::hatch::contour::march::GridValues::new(
            grid.rows, grid.cols, grid.wraps, nodes, centers,
        );
        let polylines = crate::hatch::contour::march::iso_polylines(&values, 0.0);

        assert_eq!(polylines.len(), 1, "one island has one boundary contour");
        let chain = &polylines[0];
        assert_ne!(
            chain.first().unwrap().edge,
            chain.last().unwrap().edge,
            "an island open at the lip row must not close into a loop"
        );
        for endpoint in [chain.first().unwrap().edge, chain.last().unwrap().edge] {
            assert!(
                matches!(
                    endpoint,
                    crate::hatch::contour::march::EdgeId::Horizontal { row: 0, .. }
                ),
                "endpoint {endpoint:?} should sit on the row-0 (lip) boundary"
            );
        }
    }

    #[test]
    fn outline_bbox_covers_every_corner() {
        let outline = vec![
            FacePoint::new(-1.0, 2.0),
            FacePoint::new(3.0, -4.0),
            FacePoint::new(0.5, 5.0),
        ];
        let bbox = outline_bbox(&outline);
        assert_eq!(
            bbox,
            FB::new(FacePoint::new(-1.0, -4.0), FacePoint::new(3.0, 5.0))
        );
    }
}
