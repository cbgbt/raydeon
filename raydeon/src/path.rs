use euclid::*;
use std::collections::{BTreeSet, HashSet};

/// Trait over metadata associated with each segment.
///
/// This can be used to associate material data or other arbtirary information to paths for
/// post-processing.
pub trait PathMeta: Clone + std::fmt::Debug + Send + Sync + 'static {}
impl<P> PathMeta for P where P: Clone + std::fmt::Debug + Send + Sync + 'static {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NoMetadata;

#[derive(Debug, Copy, Clone)]
pub struct LineSegment3D<Space, Metadata>
where
    Space: Copy + Clone + std::fmt::Debug,
    Metadata: PathMeta,
{
    p1: Point3D<f64, Space>,
    p2: Point3D<f64, Space>,
    norm_dir: Vector3D<f64, Space>,
    length: f64,
    meta: Metadata,
}

impl<Space> LineSegment3D<Space, NoMetadata>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub fn new(p1: Point3D<f64, Space>, p2: Point3D<f64, Space>) -> Self {
        Self::tagged(p1, p2, NoMetadata)
    }
}

impl<Space, Metadata> LineSegment3D<Space, Metadata>
where
    Space: Copy + Clone + std::fmt::Debug,
    Metadata: PathMeta,
{
    pub fn tagged(p1: Point3D<f64, Space>, p2: Point3D<f64, Space>, meta: Metadata) -> Self {
        let dir = p2 - p1;
        let length = dir.length();
        let norm_dir = dir.normalize();
        Self {
            p1,
            p2,
            length,
            norm_dir,
            meta,
        }
    }

    pub fn p1(&self) -> Point3D<f64, Space> {
        self.p1
    }

    pub fn p2(&self) -> Point3D<f64, Space> {
        self.p2
    }

    #[must_use]
    pub fn midpoint(&self) -> Point3D<f64, Space> {
        self.p1 + (self.p2 - self.p1) / 2.0
    }

    pub fn meta(&self) -> &Metadata {
        &self.meta
    }

    #[must_use]
    pub fn dir(&self) -> Vector3D<f64, Space> {
        self.norm_dir
    }

    #[must_use]
    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn cast_unit<U>(self) -> LineSegment3D<U, Metadata>
    where
        U: Copy + Clone + std::fmt::Debug,
    {
        LineSegment3D::tagged(self.p1.cast_unit(), self.p2.cast_unit(), self.meta)
    }

    pub fn xy(self) -> LineSegment2D<Space> {
        LineSegment2D::new(self.p1.xy(), self.p2.xy())
    }

    pub fn yz(self) -> LineSegment2D<Space> {
        LineSegment2D::new(self.p1.yz(), self.p2.yz())
    }

    pub fn xz(self) -> LineSegment2D<Space> {
        LineSegment2D::new(self.p1.xz(), self.p2.xz())
    }

    pub fn transform<Dst>(
        &self,
        transformation: &Transform3D<f64, Space, Dst>,
    ) -> Option<LineSegment3D<Dst, Metadata>>
    where
        Dst: Copy + Clone + std::fmt::Debug,
    {
        let (p1, p2) = (self.p1, self.p2);
        let p1t = transformation.transform_point3d(p1);
        let p2t = transformation.transform_point3d(p2);
        p1t.and_then(|p1| p2t.map(|p2| LineSegment3D::tagged(p1, p2, self.meta.clone())))
    }

    pub fn transform_without_metadata<Dst>(
        &self,
        transformation: &Transform3D<f64, Space, Dst>,
    ) -> Option<LineSegment3D<Dst, NoMetadata>>
    where
        Dst: Copy + Clone + std::fmt::Debug,
    {
        let (p1, p2) = (self.p1, self.p2);
        let p1t = transformation.transform_point3d(p1);
        let p2t = transformation.transform_point3d(p2);
        p1t.and_then(|p1| p2t.map(|p2| LineSegment3D::tagged(p1, p2, NoMetadata)))
    }
}

/// Created when a segment is chopped into several smaller pieces
pub struct SlicedSegment3D<'parent, Space, Metadata>
where
    Space: Copy + Clone + std::fmt::Debug,
    Metadata: PathMeta,
{
    num_chops: usize,
    included: BTreeSet<usize>,
    parent: &'parent LineSegment3D<Space, Metadata>,
}

impl<'parent, Space, Metadata> SlicedSegment3D<'parent, Space, Metadata>
where
    Space: Copy + Clone + std::fmt::Debug,
    Metadata: PathMeta,
{
    pub fn new(num_chops: usize, parent: &'parent LineSegment3D<Space, Metadata>) -> Self {
        let included = (0..num_chops).collect();
        Self {
            num_chops,
            included,
            parent,
        }
    }

    pub fn subsegment_len(&self) -> f64 {
        self.parent.length() / (self.num_chops as f64)
    }

    pub fn num_subsegments(&self) -> usize {
        self.included.len()
    }

    fn get_subsegment(&self, ndx: usize) -> LineSegment3D<Space, NoMetadata> {
        let segment_vec = self.parent.dir() * self.subsegment_len();
        let start = self.parent.p1 + segment_vec * (ndx as f64);
        let end = start + segment_vec;
        LineSegment3D::tagged(start, end, NoMetadata)
    }

    pub fn subsegments(&self) -> impl Iterator<Item = LineSegment3D<Space, NoMetadata>> + '_ {
        self.included
            .iter()
            .map(move |ndx| self.get_subsegment(*ndx))
    }

    pub fn remove_subsegment(&mut self, ndx: usize) {
        self.included.remove(&ndx);
    }

    pub fn join_slices(&self) -> Vec<LineSegment3D<Space, Metadata>> {
        self.join_slices_with_forgiveness(0)
    }

    /// Joins slices, ignoring gaps of size `forgiveness` or smaller
    pub fn join_slices_with_forgiveness(
        &self,
        forgiveness: usize,
    ) -> Vec<LineSegment3D<Space, Metadata>> {
        if self.included.is_empty() {
            return Vec::new();
        }
        let mut included = self.included.clone();

        let mut gap_start = None;
        let mut last_filled = None;
        for curr in 0..self.num_chops {
            let empty = !included.contains(&curr);
            match gap_start {
                Some(start) if empty && curr - start > forgiveness => gap_start = None,
                Some(start) if !empty => (start..curr).for_each(|ndx| {
                    included.insert(ndx);
                    gap_start = None
                }),
                None if empty && curr > 0 && last_filled == Some(curr - 1) => {
                    gap_start = Some(curr);
                }
                _ => (),
            }
            if !empty {
                last_filled = Some(curr);
            }
        }

        let mut ndx_groups = HashSet::new();

        let mut first = *included.first().unwrap();
        let mut last = first;

        included.iter().for_each(|ndx| {
            if *ndx == first {
                return;
            }
            if ndx - last == 1 {
                last = *ndx;
            } else {
                ndx_groups.insert(first..=last);
                first = *ndx;
                last = first;
            }
        });
        ndx_groups.insert(first..=last);

        ndx_groups
            .into_iter()
            .map(|ndx_group| {
                let start = self.get_subsegment(*ndx_group.start()).p1;
                let end = self.get_subsegment(*ndx_group.end()).p2;
                LineSegment3D::tagged(start, end, self.parent.meta.clone())
            })
            .collect()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LineSegment2D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub p1: Point2D<f64, Space>,
    pub p2: Point2D<f64, Space>,
    pub tag: usize,
}

impl<Space> LineSegment2D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub fn new(p1: Point2D<f64, Space>, p2: Point2D<f64, Space>) -> Self {
        Self::tagged(p1, p2, 0)
    }

    pub fn tagged(p1: Point2D<f64, Space>, p2: Point2D<f64, Space>, tag: usize) -> Self {
        Self { p1, p2, tag }
    }

    pub fn cast_unit<U>(self) -> LineSegment2D<U>
    where
        U: Copy + Clone + std::fmt::Debug,
    {
        LineSegment2D::new(self.p1.cast_unit(), self.p2.cast_unit())
    }

    pub fn transform<Dst>(
        &self,
        transformation: &Transform2D<f64, Space, Dst>,
    ) -> LineSegment2D<Dst>
    where
        Dst: Copy + Clone + std::fmt::Debug,
    {
        let (p1, p2) = (self.p1, self.p2);
        LineSegment2D::tagged(
            transformation.transform_point(p1),
            transformation.transform_point(p2),
            self.tag,
        )
    }
}
