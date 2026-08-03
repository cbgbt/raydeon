//! Pure marching squares over a precomputed grid of scalar samples.
//!
//! Nothing here knows about scenes, surfaces or world space — only node
//! values, an iso level, and the topology needed to walk crossings into
//! deterministic polylines. `hatch::contour::grid` builds the grid this
//! module walks; `hatch::contour` maps the polylines it returns back into
//! world-space geometry.

mod chain;
mod edge;
mod resolve;

use chain::chain_polylines;
pub(crate) use edge::{EdgeCrossing, EdgeId};
use resolve::{cell_pairs, CellCorners};
use std::collections::HashMap;

/// A row-major grid of a scalar field's samples at each lattice node, plus a
/// way to sample the field at any cell's center — consulted only to break a
/// saddle tie, so a center is never sampled unless marching actually reaches
/// an ambiguous cell.
///
/// `wraps` marks a grid whose column axis is a closed loop (a sphere's or a
/// revolution's longitude): column `cols` is the same node as column `0`.
/// Rows never wrap — a pole row or a foot/lip row is a real boundary, and a
/// contour reaching it terminates there (invariant 8).
pub(crate) struct GridValues<'a> {
    rows: usize,
    cols: usize,
    wraps: bool,
    nodes: Vec<f64>,
    center_of: Box<dyn Fn(usize, usize) -> f64 + 'a>,
}

impl std::fmt::Debug for GridValues<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GridValues")
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .field("wraps", &self.wraps)
            .field("nodes", &self.nodes)
            .finish_non_exhaustive()
    }
}

impl<'a> GridValues<'a> {
    /// `nodes` must have `rows * cols` entries, row-major. `center_of(row,
    /// col)` must give the field's value at cell `(row, col)`'s center, for
    /// any cell in the `(rows - 1) * cell_cols` grid of cells (`cell_cols`
    /// is `cols` if `wraps`, else `cols - 1`) — sampling a center is pure
    /// (invariant 4), so calling it lazily, only for the cells marching
    /// actually needs to disambiguate, changes no result.
    pub(crate) fn new(
        rows: usize,
        cols: usize,
        wraps: bool,
        nodes: Vec<f64>,
        center_of: impl Fn(usize, usize) -> f64 + 'a,
    ) -> Self {
        debug_assert_eq!(nodes.len(), rows * cols, "nodes must fill the grid exactly");
        Self {
            rows,
            cols,
            wraps,
            nodes,
            center_of: Box::new(center_of),
        }
    }

    fn cell_cols(&self) -> usize {
        if self.wraps {
            self.cols
        } else {
            self.cols - 1
        }
    }

    fn node(&self, row: usize, col: usize) -> f64 {
        self.nodes[row * self.cols + (col % self.cols)]
    }

    fn center(&self, row: usize, col: usize) -> f64 {
        (self.center_of)(row, col)
    }

    /// Gives a wrapping grid's seam one edge identity.
    ///
    /// The last cell's right edge and the first cell's left edge are the
    /// SAME physical vertical edge on a grid whose columns close into a
    /// loop (column `cols` is column `0`), but [`CellEdge::id`] mints them
    /// from raw, unwrapped `(row, col)` pairs — `Vertical { col: cols }` and
    /// `Vertical { col: 0 }` — with no knowledge of `cols` or `wraps` at that
    /// call site. Left un-canonicalized, the two cells bordering the seam
    /// disagree on the edge's identity, so a contour crossing it chains as
    /// two open ends instead of one continuous loop. Canonicalizing at mint
    /// time (here, immediately after `CellEdge::id`) rather than in the
    /// chainer keeps `iso_polylines`'s adjacency graph itself correct, so
    /// there is no post-hoc merge step to forget.
    fn canonical_edge(&self, edge: EdgeId) -> EdgeId {
        match edge {
            EdgeId::Vertical { row, col } if self.wraps && col == self.cols => {
                EdgeId::Vertical { row, col: 0 }
            }
            other => other,
        }
    }
}

/// Extracts the iso-`iso` contour of `grid` as a set of polylines.
///
/// The inside rule is STRICT (`value < iso`; `value == iso` is outside), so
/// no node sits exactly on the boundary of both — no crossing is ever
/// duplicated or dropped at an iso-equal node. A cell whose two diagonal
/// corners are inside and whose other two are outside (a saddle) is
/// otherwise ambiguous; it is split by the field's own value at the cell's
/// center, deciding which pair of corners the contour isolates.
///
/// Crossings are keyed by their exact `EdgeId` and chained by walking the
/// grid row-major, so both the polylines returned and the order of their
/// points are deterministic: the same grid and iso level always produce the
/// same output. A chain which reaches a non-wrapping grid boundary (the
/// first/last row always; the first/last column too, unless the grid
/// wraps) terminates there as one OPEN polyline with both endpoints on
/// boundary edges — it is never spuriously closed into a loop, nor split
/// into two.
pub(crate) fn iso_polylines(grid: &GridValues<'_>, iso: f64) -> Vec<Vec<EdgeCrossing>> {
    let inside = |value: f64| value < iso;

    let mut fractions: HashMap<EdgeId, f64> = HashMap::new();
    let mut adjacency: HashMap<EdgeId, Vec<EdgeId>> = HashMap::new();
    let connect = |a: EdgeId, b: EdgeId, adjacency: &mut HashMap<EdgeId, Vec<EdgeId>>| {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    };

    for row in 0..grid.rows.saturating_sub(1) {
        for col in 0..grid.cell_cols() {
            let nw = grid.node(row, col);
            let ne = grid.node(row, col + 1);
            let se = grid.node(row + 1, col + 1);
            let sw = grid.node(row + 1, col);
            let (nw_in, ne_in, se_in, sw_in) = (inside(nw), inside(ne), inside(se), inside(sw));
            if nw_in == ne_in && ne_in == se_in && se_in == sw_in {
                continue;
            }

            let corners = CellCorners {
                nw: nw_in,
                ne: ne_in,
                se: se_in,
                sw: sw_in,
            };
            // Only a saddle's two branches call this; every other corner
            // pattern resolves without ever sampling the cell's center.
            for (a, b) in cell_pairs(corners, || inside(grid.center(row, col))) {
                let edge_a = grid.canonical_edge(a.id(row, col));
                let edge_b = grid.canonical_edge(b.id(row, col));
                fractions
                    .entry(edge_a)
                    .or_insert_with(|| edge_fraction(grid, edge_a, iso));
                fractions
                    .entry(edge_b)
                    .or_insert_with(|| edge_fraction(grid, edge_b, iso));
                connect(edge_a, edge_b, &mut adjacency);
            }
        }
    }

    chain_polylines(&adjacency, &fractions)
}

/// How far from its first node toward its second (`EdgeId::nodes`) the iso
/// level crosses `edge`.
///
/// The two node values always differ: one is `< iso` and the other is
/// `>= iso` by construction (this edge was reported as crossing), so the
/// denominator is never zero.
fn edge_fraction(grid: &GridValues<'_>, edge: EdgeId, iso: f64) -> f64 {
    let ((r0, c0), (r1, c1)) = edge.nodes();
    let a = grid.node(r0, c0);
    let b = grid.node(r1, c1);
    (iso - a) / (b - a)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-wrapping grid of `rows` by `cols` nodes, sampling `field` at
    /// each node, and lazily at each cell's center (the midpoint of its four
    /// corners' `(row, col)` coordinates — exact for the affine and radial
    /// fields these tests use) — exercising the same on-demand center
    /// sampling the contour engine relies on.
    fn grid_of(
        rows: usize,
        cols: usize,
        field: impl Fn(f64, f64) -> f64 + 'static,
    ) -> GridValues<'static> {
        let nodes: Vec<f64> = (0..rows)
            .flat_map(|row| (0..cols).map(move |col| (row, col)))
            .map(|(row, col)| field(row as f64, col as f64))
            .collect();
        GridValues::new(rows, cols, false, nodes, move |row, col| {
            field(row as f64 + 0.5, col as f64 + 0.5)
        })
    }

    fn is_closed(polyline: &[EdgeCrossing]) -> bool {
        polyline.len() > 1 && polyline.first().unwrap().edge == polyline.last().unwrap().edge
    }

    #[test]
    fn a_circular_field_extracts_one_closed_loop_near_the_true_radius() {
        let center = (10.0, 10.0);
        let radius = 6.0;
        let grid = grid_of(21, 21, move |row, col| {
            ((row - center.0).powi(2) + (col - center.1).powi(2)).sqrt()
        });

        let polylines = iso_polylines(&grid, radius);
        assert_eq!(
            polylines.len(),
            1,
            "a circle well inside the grid is one loop"
        );
        let loop_ = &polylines[0];
        assert!(
            is_closed(loop_),
            "a field with no boundary crossing closes on itself"
        );

        for crossing in loop_ {
            let ((r0, c0), (r1, c1)) = crossing.edge.nodes();
            let lerp = |a: f64, b: f64| a + (b - a) * crossing.fraction;
            let (row, col) = (lerp(r0 as f64, r1 as f64), lerp(c0 as f64, c1 as f64));
            let sampled_radius = ((row - center.0).powi(2) + (col - center.1).powi(2)).sqrt();
            assert!(
                (sampled_radius - radius).abs() < 1.5,
                "crossing at radius {sampled_radius}, expected near {radius} (within a grid cell)"
            );
        }
    }

    #[test]
    fn a_linear_field_crossing_the_grid_yields_one_open_polyline_on_the_boundary() {
        // Increases left to right, uniform top to bottom: the iso level cuts
        // one column boundary straight down every row, top row to bottom
        // row — an open chain the whole height of the (non-wrapping) grid.
        let grid = grid_of(6, 10, |_row, col| col);

        let polylines = iso_polylines(&grid, 4.5);
        assert_eq!(polylines.len(), 1, "one band crosses the whole grid");
        let chain = &polylines[0];
        assert!(
            !is_closed(chain),
            "a field with no wrap must not close into a loop"
        );
        assert_eq!(chain.len(), 6, "one crossing per row, top to bottom");

        // Every crossing lies on the same Horizontal edge column (row-to-row
        // uniform value never differs along a column); the two ends of the
        // open chain are the top and bottom BOUNDARY rows.
        let first = chain.first().unwrap().edge;
        let last = chain.last().unwrap().edge;
        for endpoint in [first, last] {
            assert!(
                matches!(endpoint, EdgeId::Horizontal { row, .. } if row == 0 || row == 5),
                "endpoint {endpoint:?} should sit on a top/bottom boundary row"
            );
        }
    }

    #[test]
    fn an_iso_equal_node_counts_as_outside_with_no_duplicated_or_dropped_crossing() {
        // The middle column sits exactly on the iso level: strict `< iso`
        // makes it outside, so each row reads as one inside corner (the
        // left node), not two, and not zero.
        let grid = GridValues::new(
            2,
            3,
            false,
            vec![0.0, 1.0, 2.0, 0.0, 1.0, 2.0],
            |_row, col| if col == 0 { 0.5 } else { 1.5 },
        );

        let polylines = iso_polylines(&grid, 1.0);
        let total_crossings: usize = polylines.iter().map(Vec::len).sum();
        // Exactly one cell has a crossing (the left one, nw=0<1 inside,
        // ne=1 not<1 outside); the right cell has both corners at/over 1.0,
        // neither inside, so it contributes nothing.
        assert_eq!(polylines.len(), 1);
        assert_eq!(
            total_crossings, 2,
            "one open two-point chain, no duplicate crossing"
        );
    }

    #[test]
    fn saddle_cells_pick_a_pairing_deterministically_from_the_center_sample() {
        // nw and se inside, ne and sw outside: a diagonal saddle.
        let saddle = |center: f64| {
            GridValues::new(2, 2, false, vec![-1.0, 1.0, 1.0, -1.0], move |_, _| center)
        };

        let center_inside = iso_polylines(&saddle(-1.0), 0.0);
        let center_outside = iso_polylines(&saddle(1.0), 0.0);

        // Both are two separate open two-point chains (all four crossings
        // sit on this grid's only cell, so every edge is a grid boundary).
        assert_eq!(center_inside.len(), 2);
        assert_eq!(center_outside.len(), 2);

        let edge_sets = |polylines: &[Vec<EdgeCrossing>]| -> Vec<Vec<EdgeId>> {
            let mut sets: Vec<Vec<EdgeId>> = polylines
                .iter()
                .map(|chain| {
                    let mut edges: Vec<EdgeId> = chain.iter().map(|c| c.edge).collect();
                    edges.sort();
                    edges
                })
                .collect();
            sets.sort();
            sets
        };

        assert_ne!(
            edge_sets(&center_inside),
            edge_sets(&center_outside),
            "the two center readings must pick different pairings"
        );

        // Repeating either reading must reproduce the exact same output.
        assert_eq!(iso_polylines(&saddle(-1.0), 0.0), center_inside);
        assert_eq!(iso_polylines(&saddle(1.0), 0.0), center_outside);
    }

    /// A wrapping grid of `rows` by `cols` nodes: `field` is sampled with a
    /// column argument already reduced circularly (period `cols`), so a
    /// caller may pass any real column — including the seam cell's center at
    /// `cols - 0.5`, the circular midpoint between column `cols - 1` and
    /// column `0` — and get the physically consistent value.
    fn wrapping_grid_of(
        rows: usize,
        cols: usize,
        field: impl Fn(f64, f64) -> f64 + 'static,
    ) -> GridValues<'static> {
        let nodes: Vec<f64> = (0..rows)
            .flat_map(|row| (0..cols).map(move |col| (row, col)))
            .map(|(row, col)| field(row as f64, col as f64))
            .collect();
        GridValues::new(rows, cols, true, nodes, move |row, col| {
            field(row as f64 + 0.5, col as f64 + 0.5)
        })
    }

    /// The circular distance from `col` to `target`, in a column space of
    /// period `cols` — the shortest way around the wrap either direction.
    fn circular_col_dist(col: f64, target: f64, cols: f64) -> f64 {
        let raw = (col - target).rem_euclid(cols);
        raw.min(cols - raw)
    }

    #[test]
    fn a_field_straddling_the_phi_seam_closes_into_one_loop() {
        // A circular blob centered at row 10, column 0 — the seam column
        // itself — on a 21-row by 24-column wrapping grid. Its boundary
        // necessarily crosses the seam's vertical edge (column 0/24) at top
        // and bottom, which is exactly the case the seam-identity bug
        // breaks: without canonicalization this comes out as two open
        // chains, one per side of the seam, instead of one closed loop.
        let cols = 24.0;
        let radius = 4.0;
        let grid = wrapping_grid_of(21, 24, move |row, col| {
            let dr = row - 10.0;
            let dc = circular_col_dist(col, 0.0, cols);
            (dr * dr + dc * dc).sqrt()
        });

        let polylines = iso_polylines(&grid, radius);
        assert_eq!(
            polylines.len(),
            1,
            "a blob straddling the seam is one loop, not two open chains"
        );
        assert!(
            is_closed(&polylines[0]),
            "a loop crossing the phi seam must still close on itself"
        );
    }

    #[test]
    fn a_loop_crossing_the_phi_seam_twice_stays_one_loop() {
        // A long horizontal band centered on row 10 that runs the full
        // column wrap TWICE in effect: it is inside for every column (a
        // full ring around the cylinder) between two row bounds, so its
        // single closed contour (following the band's near edge, then
        // wrapping the seam, then the far edge, then wrapping the seam
        // again to close) crosses the seam's vertical edge twice yet must
        // still chain as one loop, not split into two.
        let grid = wrapping_grid_of(21, 24, |row, _col| (row - 10.0).abs());

        let polylines = iso_polylines(&grid, 3.0);
        assert_eq!(
            polylines.len(),
            2,
            "a band with two boundaries (top and bottom of the ring) is two loops"
        );
        for loop_ in &polylines {
            assert!(
                is_closed(loop_),
                "each boundary of a full-wrap band closes into its own loop, \
                 crossing the seam without splitting"
            );
        }
    }

    #[test]
    fn extraction_is_deterministic_across_repeated_runs() {
        let grid = grid_of(9, 13, |row, col| {
            ((row - 4.0).powi(2) + (col - 6.0).powi(2)).sqrt()
        });
        let first = iso_polylines(&grid, 3.0);
        let second = iso_polylines(&grid, 3.0);
        assert_eq!(first, second);
    }
}
