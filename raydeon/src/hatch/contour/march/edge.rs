//! Grid edge identity: naming a marching-squares cell's four sides so the
//! (up to) two cells bordering an edge agree on the same identity, and
//! recording where the iso level crosses one.

/// A grid edge, identified by its two endpoint nodes — not by which cell
/// asked for it — so that the (up to) two cells bordering an edge agree on
/// the same identity. That shared identity is what lets crossings chain
/// across a cell boundary into one polyline.
///
/// Ordering is arbitrary but total and fixed, which is all determinism
/// needs: it lets [`super::iso_polylines`] pick chain-starting points from a
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
pub(super) enum CellEdge {
    Top,
    Right,
    Bottom,
    Left,
}

impl CellEdge {
    pub(super) fn id(self, row: usize, col: usize) -> EdgeId {
        match self {
            CellEdge::Top => EdgeId::Horizontal { row, col },
            CellEdge::Bottom => EdgeId::Horizontal { row: row + 1, col },
            CellEdge::Left => EdgeId::Vertical { row, col },
            CellEdge::Right => EdgeId::Vertical { row, col: col + 1 },
        }
    }
}
