//! Pure marching squares over a precomputed grid of scalar samples.
//!
//! Nothing here knows about scenes, surfaces or world space — only node
//! values, an iso level, and the topology needed to walk crossings into
//! deterministic polylines. `hatch::contour::grid` builds the grid this
//! module walks; `hatch::contour` maps the polylines it returns back into
//! world-space geometry.

use std::collections::{HashMap, HashSet};

/// A row-major grid of a scalar field's samples at each lattice node, plus
/// its value at every cell's center (consulted only to break a saddle tie).
///
/// `wraps` marks a grid whose column axis is a closed loop (a sphere's or a
/// revolution's longitude): column `cols` is the same node as column `0`.
/// Rows never wrap — a pole row or a foot/lip row is a real boundary, and a
/// contour reaching it terminates there (invariant 8).
#[derive(Debug, Clone)]
pub(crate) struct GridValues {
    rows: usize,
    cols: usize,
    wraps: bool,
    nodes: Vec<f64>,
    centers: Vec<f64>,
}

impl GridValues {
    /// `nodes` must have `rows * cols` entries, row-major. `centers` must
    /// have one entry per cell, row-major: `(rows - 1) * cell_cols` of them,
    /// where `cell_cols` is `cols` if `wraps`, else `cols - 1`.
    pub(crate) fn new(
        rows: usize,
        cols: usize,
        wraps: bool,
        nodes: Vec<f64>,
        centers: Vec<f64>,
    ) -> Self {
        debug_assert_eq!(nodes.len(), rows * cols, "nodes must fill the grid exactly");
        let cell_cols = if wraps { cols } else { cols.saturating_sub(1) };
        debug_assert_eq!(
            centers.len(),
            rows.saturating_sub(1) * cell_cols,
            "one center sample per cell"
        );
        Self {
            rows,
            cols,
            wraps,
            nodes,
            centers,
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
        self.centers[row * self.cell_cols() + col]
    }
}

/// A grid edge, identified by its two endpoint nodes — not by which cell
/// asked for it — so that the (up to) two cells bordering an edge agree on
/// the same identity. That shared identity is what lets crossings chain
/// across a cell boundary into one polyline.
///
/// Ordering is arbitrary but total and fixed, which is all determinism
/// needs: it lets [`iso_polylines`] pick chain-starting points from a
/// canonical scan order instead of hash-map iteration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum EdgeId {
    /// Between node `(row, col)` and node `(row, col + 1)` — the direction
    /// that wraps, on a grid whose columns are a closed loop.
    Horizontal { row: usize, col: usize },
    /// Between node `(row, col)` and node `(row + 1, col)` — never wraps.
    Vertical { row: usize, col: usize },
}

impl EdgeId {
    /// The raw `(row, col)` of this edge's two endpoint nodes, in the order
    /// [`EdgeCrossing::fraction`] is measured from and toward.
    pub(crate) fn nodes(self) -> ((usize, usize), (usize, usize)) {
        match self {
            EdgeId::Horizontal { row, col } => ((row, col), (row, col + 1)),
            EdgeId::Vertical { row, col } => ((row, col), (row + 1, col)),
        }
    }
}

/// A point where the iso level crosses one grid edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EdgeCrossing {
    pub(crate) edge: EdgeId,
    /// How far along the edge, from its first node toward its second
    /// (`EdgeId::nodes`), the crossing sits — in `[0, 1]`.
    pub(crate) fraction: f64,
}

/// The four edges of one marching-squares cell, named by compass position.
#[derive(Debug, Clone, Copy)]
enum CellEdge {
    Top,
    Right,
    Bottom,
    Left,
}

impl CellEdge {
    fn id(self, row: usize, col: usize) -> EdgeId {
        match self {
            CellEdge::Top => EdgeId::Horizontal { row, col },
            CellEdge::Bottom => EdgeId::Horizontal { row: row + 1, col },
            CellEdge::Left => EdgeId::Vertical { row, col },
            CellEdge::Right => EdgeId::Vertical { row, col: col + 1 },
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
pub(crate) fn iso_polylines(grid: &GridValues, iso: f64) -> Vec<Vec<EdgeCrossing>> {
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

            let center_inside = inside(grid.center(row, col));
            let corners = CellCorners {
                nw: nw_in,
                ne: ne_in,
                se: se_in,
                sw: sw_in,
            };
            for (a, b) in cell_pairs(corners, center_inside) {
                let edge_a = a.id(row, col);
                let edge_b = b.id(row, col);
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
fn edge_fraction(grid: &GridValues, edge: EdgeId, iso: f64) -> f64 {
    let ((r0, c0), (r1, c1)) = edge.nodes();
    let a = grid.node(r0, c0);
    let b = grid.node(r1, c1);
    (iso - a) / (b - a)
}

/// Which of a cell's four corners are inside the iso level, named by
/// compass position.
#[derive(Debug, Clone, Copy)]
struct CellCorners {
    nw: bool,
    ne: bool,
    se: bool,
    sw: bool,
}

/// The pairs of edges one cell's crossings connect, given its `corners` and,
/// for the two ambiguous diagonal cases, whether the cell's center sample is
/// inside the iso level.
///
/// Exhaustive over all 16 corner combinations: the two saddle arms are the
/// only ones a single reading of the corners cannot resolve alone.
fn cell_pairs(corners: CellCorners, center_inside: bool) -> Vec<(CellEdge, CellEdge)> {
    use CellEdge::{Bottom, Left, Right, Top};
    let CellCorners { nw, ne, se, sw } = corners;
    match (nw, ne, se, sw) {
        (false, false, false, false) | (true, true, true, true) => vec![],
        // One inside corner, or its complement (three inside): the same
        // pair of edges crosses either way, isolating the singular corner.
        (true, false, false, false) | (false, true, true, true) => vec![(Top, Left)],
        (false, true, false, false) | (true, false, true, true) => vec![(Top, Right)],
        (false, false, true, false) | (true, true, false, true) => vec![(Right, Bottom)],
        (false, false, false, true) | (true, true, true, false) => vec![(Bottom, Left)],
        // Two adjacent inside corners: one unambiguous pair.
        (true, true, false, false) | (false, false, true, true) => vec![(Left, Right)],
        (false, true, true, false) | (true, false, false, true) => vec![(Top, Bottom)],
        // Saddles: diagonal corners share status, so both readings of the
        // four crossing edges are geometrically valid; the center sample
        // picks the one consistent with the field in between.
        (true, false, true, false) => {
            if center_inside {
                vec![(Top, Right), (Bottom, Left)]
            } else {
                vec![(Top, Left), (Right, Bottom)]
            }
        }
        (false, true, false, true) => {
            if center_inside {
                vec![(Top, Left), (Right, Bottom)]
            } else {
                vec![(Top, Right), (Bottom, Left)]
            }
        }
    }
}

/// Walks the crossing-adjacency graph into polylines, in a fixed order: open
/// chains first (their far endpoint is the crossing the walk stops at), each
/// started from its lowest-`EdgeId` endpoint; then whatever closed loops
/// remain, each started from its lowest-`EdgeId` crossing. Only the
/// grid-derived `EdgeId` ordering decides where any polyline starts —
/// never hash-map iteration order — so the result is reproducible.
fn chain_polylines(
    adjacency: &HashMap<EdgeId, Vec<EdgeId>>,
    fractions: &HashMap<EdgeId, f64>,
) -> Vec<Vec<EdgeCrossing>> {
    let mut ids: Vec<EdgeId> = adjacency.keys().copied().collect();
    ids.sort();

    let mut visited: HashSet<EdgeId> = HashSet::new();
    let mut polylines = Vec::new();

    let extract =
        |start: EdgeId, visited: &mut HashSet<EdgeId>, polylines: &mut Vec<Vec<EdgeCrossing>>| {
            let chain = walk_chain(adjacency, start);
            for &id in &chain {
                visited.insert(id);
            }
            polylines.push(
                chain
                    .into_iter()
                    .map(|edge| EdgeCrossing {
                        edge,
                        fraction: fractions[&edge],
                    })
                    .collect(),
            );
        };

    for &id in &ids {
        if !visited.contains(&id) && adjacency[&id].len() == 1 {
            extract(id, &mut visited, &mut polylines);
        }
    }
    for &id in &ids {
        if !visited.contains(&id) {
            extract(id, &mut visited, &mut polylines);
        }
    }

    polylines
}

/// Walks from `start`, always stepping to the neighbor which is not where
/// the walk just came from. Stops when it returns to `start` (closed loop —
/// `start` is pushed again, so the chain's first and last entries match) or
/// when it reaches another degree-one crossing (an open chain's far end).
fn walk_chain(adjacency: &HashMap<EdgeId, Vec<EdgeId>>, start: EdgeId) -> Vec<EdgeId> {
    let mut chain = vec![start];
    let mut prev: Option<EdgeId> = None;
    let mut current = start;

    loop {
        let neighbors = &adjacency[&current];
        let next = match (neighbors.len(), prev) {
            (1, None) | (2, None) => neighbors[0],
            (1, Some(_)) => break,
            (2, Some(from)) => {
                if neighbors[0] == from {
                    neighbors[1]
                } else {
                    neighbors[0]
                }
            }
            _ => unreachable!("a marching-squares crossing borders at most two cells"),
        };

        chain.push(next);
        if next == start {
            break;
        }
        prev = Some(current);
        current = next;
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-wrapping grid of `rows` by `cols` nodes, sampling `field` at
    /// each node and at each cell's center (the midpoint of its four
    /// corners' `(row, col)` coordinates — exact for the affine and radial
    /// fields these tests use).
    fn grid_of(rows: usize, cols: usize, field: impl Fn(f64, f64) -> f64) -> GridValues {
        let nodes: Vec<f64> = (0..rows)
            .flat_map(|row| (0..cols).map(move |col| (row, col)))
            .map(|(row, col)| field(row as f64, col as f64))
            .collect();
        let centers: Vec<f64> = (0..rows - 1)
            .flat_map(|row| (0..cols - 1).map(move |col| (row, col)))
            .map(|(row, col)| field(row as f64 + 0.5, col as f64 + 0.5))
            .collect();
        GridValues::new(rows, cols, false, nodes, centers)
    }

    fn is_closed(polyline: &[EdgeCrossing]) -> bool {
        polyline.len() > 1 && polyline.first().unwrap().edge == polyline.last().unwrap().edge
    }

    #[test]
    fn a_circular_field_extracts_one_closed_loop_near_the_true_radius() {
        let center = (10.0, 10.0);
        let radius = 6.0;
        let grid = grid_of(21, 21, |row, col| {
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
            vec![0.5, 1.5],
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
        let saddle =
            |center: f64| GridValues::new(2, 2, false, vec![-1.0, 1.0, 1.0, -1.0], vec![center]);

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
