use bon::Builder;
use euclid::*;
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Copy, Clone, Builder)]
#[builder(start_fn(name = new))]
pub struct LineSegment3D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    #[builder(into)]
    p1: Point3D<f64, Space>,
    #[builder(into)]
    p2: Point3D<f64, Space>,

    #[builder(skip = (p2 - p1).normalize())]
    norm_dir: Vector3D<f64, Space>,
    #[builder(skip = (p2 - p1).length())]
    length: f64,
}

impl<Space> LineSegment3D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub fn new_segment(
        p1: impl Into<Point3D<f64, Space>>,
        p2: impl Into<Point3D<f64, Space>>,
    ) -> Self {
        Self::new().p1(p1).p2(p2).build()
    }

    pub fn from_points(
        pairs: Vec<(
            impl Into<Point3D<f64, Space>>,
            impl Into<Point3D<f64, Space>>,
        )>,
    ) -> Vec<Self> {
        pairs
            .into_iter()
            .map(|(p1, p2)| LineSegment3D::new_segment(p1, p2))
            .collect()
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

    #[must_use]
    pub fn dir(&self) -> Vector3D<f64, Space> {
        self.norm_dir
    }

    #[must_use]
    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn cast_unit<U>(self) -> LineSegment3D<U>
    where
        U: Copy + Clone + std::fmt::Debug,
    {
        LineSegment3D::new()
            .p1(self.p1.cast_unit())
            .p2(self.p2.cast_unit())
            .build()
    }

    pub fn xy(self) -> LineSegment2D<Space> {
        LineSegment2D::new()
            .p1(self.p1.xy())
            .p2(self.p2.xy())
            .build()
    }

    pub fn transform<Dst>(
        &self,
        transformation: &Transform3D<f64, Space, Dst>,
    ) -> Option<LineSegment3D<Dst>>
    where
        Dst: Copy + Clone + std::fmt::Debug,
    {
        let (p1, p2) = (self.p1, self.p2);
        let p1t = transformation.transform_point3d(p1);
        let p2t = transformation.transform_point3d(p2);
        p1t.and_then(|p1| p2t.map(|p2| LineSegment3D::new().p1(p1).p2(p2).build()))
    }
}

/// Created when a segment is chopped into several smaller pieces
pub struct SlicedSegment3D<'parent, Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    num_chops: usize,
    included: BTreeSet<usize>,
    parent: &'parent LineSegment3D<Space>,
}

impl<'parent, Space> SlicedSegment3D<'parent, Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub fn new(num_chops: usize, parent: &'parent LineSegment3D<Space>) -> Self {
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

    fn get_subsegment(&self, ndx: usize) -> LineSegment3D<Space> {
        let segment_vec = self.parent.dir() * self.subsegment_len();
        let start = self.parent.p1 + segment_vec * (ndx as f64);
        let end = start + segment_vec;
        LineSegment3D::new().p1(start).p2(end).build()
    }

    pub fn subsegments(&self) -> impl Iterator<Item = LineSegment3D<Space>> + '_ {
        self.included
            .iter()
            .map(move |ndx| self.get_subsegment(*ndx))
    }

    pub fn remove_subsegment(&mut self, ndx: usize) {
        self.included.remove(&ndx);
    }

    pub fn join_slices(&self) -> Vec<LineSegment3D<Space>> {
        self.join_slices_with_forgiveness(0)
    }

    /// Joins slices, ignoring gaps of size `forgiveness` or smaller
    pub fn join_slices_with_forgiveness(&self, forgiveness: usize) -> Vec<LineSegment3D<Space>> {
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
                LineSegment3D::new().p1(start).p2(end).build()
            })
            .collect()
    }
}

#[derive(Debug, Copy, Clone, Builder)]
#[builder(start_fn(name = new))]
pub struct LineSegment2D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    #[builder(into)]
    pub p1: Point2D<f64, Space>,
    #[builder(into)]
    pub p2: Point2D<f64, Space>,
}

impl<Space> LineSegment2D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub fn new_segment(p1: Point2D<f64, Space>, p2: Point2D<f64, Space>) -> Self {
        Self::new().p1(p1).p2(p2).build()
    }

    pub fn transform<Dst>(
        &self,
        transformation: &Transform2D<f64, Space, Dst>,
    ) -> LineSegment2D<Dst>
    where
        Dst: Copy + Clone + std::fmt::Debug,
    {
        LineSegment2D::new()
            .p1(transformation.transform_point(self.p1))
            .p2(transformation.transform_point(self.p2))
            .build()
    }

    pub fn to_3d(&self) -> LineSegment3D<Space> {
        LineSegment3D::new()
            .p1(self.p1.to_3d())
            .p2(self.p2.to_3d())
            .build()
    }
}
