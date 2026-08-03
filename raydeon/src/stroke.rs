//! The output vocabulary of a render: pen strokes, grouped by the pen which
//! draws them.

use crate::{CameraSpace, Point2};
use std::collections::BTreeSet;

/// Pen/layer identity for multi-color plots.
///
/// Shapes without a material, and strokes which belong to no shape in
/// particular, plot with the default pen.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct PenId(usize);

impl PenId {
    pub fn new(pen: usize) -> Self {
        Self(pen)
    }

    pub fn value(&self) -> usize {
        self.0
    }
}

/// What role a stroke plays in the drawing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StrokeKind {
    /// Geometry edges of the scene's shapes.
    Outline,
    /// Shading strokes.
    Hatch,
    /// Iso-contour strokes: per-material tone and silhouette outlines.
    Contour,
}

/// One camera-space pen stroke of the finished drawing.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Stroke {
    pub p1: Point2<CameraSpace>,
    pub p2: Point2<CameraSpace>,
    pub pen: PenId,
    pub kind: StrokeKind,
}

/// The output of a render.
///
/// Strokes may be consumed flat, in a deterministic order, or grouped by the
/// pen which draws them, which is what a multi-pen plotter needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendering {
    strokes: Vec<Stroke>,
}

impl Rendering {
    pub(crate) fn new(strokes: Vec<Stroke>) -> Self {
        Self { strokes }
    }

    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
    }

    /// Every pen this drawing needs, in ascending order.
    pub fn pens(&self) -> BTreeSet<PenId> {
        self.strokes.iter().map(|stroke| stroke.pen).collect()
    }

    pub fn strokes_for_pen(&self, pen: PenId) -> impl Iterator<Item = &Stroke> + '_ {
        self.strokes.iter().filter(move |stroke| stroke.pen == pen)
    }
}
