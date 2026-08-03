//! Resolving one marching-squares cell's corner pattern into the pairs of
//! edges its crossings connect — the one place that decides a saddle by its
//! center sample.

use super::edge::CellEdge;

/// Which of a cell's four corners are inside the iso level, named by
/// compass position.
#[derive(Debug, Clone, Copy)]
pub(super) struct CellCorners {
    pub(super) nw: bool,
    pub(super) ne: bool,
    pub(super) se: bool,
    pub(super) sw: bool,
}

/// The pairs of edges one cell's crossings connect, given its `corners` and,
/// for the two ambiguous diagonal cases only, whether the cell's center
/// sample is inside the iso level — `center_inside` is called at most once,
/// and only from those two saddle arms, so a non-saddle cell never samples
/// its center at all.
///
/// Exhaustive over all 16 corner combinations: the two saddle arms are the
/// only ones a single reading of the corners cannot resolve alone.
pub(super) fn cell_pairs(
    corners: CellCorners,
    center_inside: impl FnOnce() -> bool,
) -> Vec<(CellEdge, CellEdge)> {
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
            if center_inside() {
                vec![(Top, Right), (Bottom, Left)]
            } else {
                vec![(Top, Left), (Right, Bottom)]
            }
        }
        (false, true, false, true) => {
            if center_inside() {
                vec![(Top, Left), (Right, Bottom)]
            } else {
                vec![(Top, Right), (Bottom, Left)]
            }
        }
    }
}
