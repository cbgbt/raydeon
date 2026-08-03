//! Walking a crossing-adjacency graph into deterministic polylines.
//!
//! Nothing here classifies a cell — it only knows crossings and which other
//! crossings each one connects to (`super::iso_polylines` builds that graph
//! from `resolve::cell_pairs`), and turns it into ordered chains.

use super::edge::{EdgeCrossing, EdgeId};
use std::collections::{HashMap, HashSet};

/// Walks the crossing-adjacency graph into polylines, in a fixed order: open
/// chains first (their far endpoint is the crossing the walk stops at), each
/// started from its lowest-`EdgeId` endpoint; then whatever closed loops
/// remain, each started from its lowest-`EdgeId` crossing. Only the
/// grid-derived `EdgeId` ordering decides where any polyline starts —
/// never hash-map iteration order — so the result is reproducible.
pub(super) fn chain_polylines(
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
