use bon::Builder;
use euclid::*;
use std::collections::BTreeSet;

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

    /// Joins each maximal run of adjacent surviving slices into one segment.
    pub fn join_slices(&self) -> Vec<LineSegment3D<Space>> {
        self.join_slices_with_forgiveness(0)
    }

    /// Joins the surviving slices into segments, bridging gaps of at most
    /// `forgiveness` removed slices.
    ///
    /// A gap of exactly `forgiveness` slices is bridged, a gap of
    /// `forgiveness + 1` is preserved. A forgiveness of `0` therefore bridges
    /// nothing: it joins runs of adjacent slices and leaves every gap intact.
    ///
    /// Segments come out ordered by ascending slice index, which is what lets a
    /// render produce the same ordered stroke sequence every time.
    pub fn join_slices_with_forgiveness(&self, forgiveness: usize) -> Vec<LineSegment3D<Space>> {
        let mut spans: Vec<(usize, usize)> = Vec::new();

        // `included` iterates in ascending order, so each index either extends
        // the span in progress or starts a new one.
        for &ndx in self.included.iter() {
            match spans.last_mut() {
                Some((_, end)) if ndx - *end - 1 <= forgiveness => *end = ndx,
                _ => spans.push((ndx, ndx)),
            }
        }

        spans
            .into_iter()
            .map(|(first, last)| {
                let start = self.get_subsegment(first).p1;
                let end = self.get_subsegment(last).p2;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldSpace;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    /// A parent segment whose slices are the unit intervals `[i, i + 1]` along
    /// x, so a joined segment's endpoints read back as slice indices.
    fn unit_parent(num_chops: usize) -> LineSegment3D<WorldSpace> {
        LineSegment3D::new_segment((0.0, 0.0, 0.0), (num_chops as f64, 0.0, 0.0))
    }

    fn slice_with<'p>(
        parent: &'p LineSegment3D<WorldSpace>,
        num_chops: usize,
        kept: &BTreeSet<usize>,
    ) -> SlicedSegment3D<'p, WorldSpace> {
        let mut sliced = SlicedSegment3D::new(num_chops, parent);
        (0..num_chops)
            .filter(|ndx| !kept.contains(ndx))
            .for_each(|ndx| sliced.remove_subsegment(ndx));
        sliced
    }

    /// Recovers the slice index span each joined segment covers.
    fn spans_of(joined: &[LineSegment3D<WorldSpace>]) -> Vec<(usize, usize)> {
        joined
            .iter()
            .map(|segment| {
                let first = segment.p1().x;
                let last = segment.p2().x - 1.0;
                assert_eq!(first, first.round(), "segment starts mid-slice");
                assert_eq!(last, last.round(), "segment ends mid-slice");
                (first as usize, last as usize)
            })
            .collect()
    }

    fn join(num_chops: usize, kept: &BTreeSet<usize>, forgiveness: usize) -> Vec<(usize, usize)> {
        let parent = unit_parent(num_chops);
        let joined = slice_with(&parent, num_chops, kept).join_slices_with_forgiveness(forgiveness);
        spans_of(&joined)
    }

    /// Slice indices to keep, drawn as a subset of `0..num_chops`.
    fn kept_strategy() -> impl Strategy<Value = (usize, BTreeSet<usize>)> {
        (1usize..24).prop_flat_map(|num_chops| {
            proptest::collection::vec(any::<bool>(), num_chops).prop_map(move |mask| {
                let kept = mask
                    .iter()
                    .enumerate()
                    .filter_map(|(ndx, keep)| keep.then_some(ndx))
                    .collect();
                (num_chops, kept)
            })
        })
    }

    proptest! {
        /// Joined segments are ordered by ascending slice index and never overlap.
        #[test]
        fn joins_in_ascending_order((num_chops, kept) in kept_strategy(), forgiveness in 0usize..6) {
            let spans = join(num_chops, &kept, forgiveness);
            for (first, last) in spans.iter() {
                prop_assert!(first <= last);
            }
            for pair in spans.windows(2) {
                prop_assert!(pair[0].1 < pair[1].0, "spans {:?} out of order", pair);
            }
        }

        /// Gaps larger than the forgiveness survive as separate segments.
        #[test]
        fn preserves_gaps_beyond_forgiveness(
            (num_chops, kept) in kept_strategy(),
            forgiveness in 0usize..6,
        ) {
            let spans = join(num_chops, &kept, forgiveness);
            for pair in spans.windows(2) {
                let gap = pair[1].0 - pair[0].1 - 1;
                prop_assert!(gap > forgiveness, "gap of {} bridged with forgiveness {}", gap, forgiveness);
            }
        }

        /// A segment starts and ends on a surviving slice, and every gap it
        /// spans over is within the forgiveness.
        #[test]
        fn bridges_only_within_forgiveness(
            (num_chops, kept) in kept_strategy(),
            forgiveness in 0usize..6,
        ) {
            let spans = join(num_chops, &kept, forgiveness);
            for &(first, last) in spans.iter() {
                prop_assert!(kept.contains(&first), "segment starts on a removed slice");
                prop_assert!(kept.contains(&last), "segment ends on a removed slice");

                let mut gap = 0;
                for ndx in first..=last {
                    gap = if kept.contains(&ndx) { 0 } else { gap + 1 };
                    prop_assert!(gap <= forgiveness, "bridged a gap of {}", gap);
                }
            }
        }

        /// Every surviving slice ends up in exactly one segment.
        #[test]
        fn covers_every_surviving_slice(
            (num_chops, kept) in kept_strategy(),
            forgiveness in 0usize..6,
        ) {
            let spans = join(num_chops, &kept, forgiveness);
            for ndx in kept.iter() {
                let covering = spans.iter().filter(|(first, last)| (first..=last).contains(&ndx)).count();
                prop_assert_eq!(covering, 1, "slice {} covered {} times", ndx, covering);
            }
        }

        /// With no forgiveness, joining only merges adjacent slices: the joined
        /// length is exactly the length of the slices which survived.
        #[test]
        fn forgiveness_zero_bridges_nothing((num_chops, kept) in kept_strategy()) {
            let parent = unit_parent(num_chops);
            let joined = slice_with(&parent, num_chops, &kept).join_slices();
            let total: f64 = joined.iter().map(LineSegment3D::length).sum();
            prop_assert_eq!(total, kept.len() as f64);

            for &(first, last) in spans_of(&joined).iter() {
                for ndx in first..=last {
                    prop_assert!(kept.contains(&ndx), "slice {} bridged with no forgiveness", ndx);
                }
            }
        }
    }

    #[test]
    fn bridges_a_gap_of_exactly_the_forgiveness() {
        let kept = BTreeSet::from([0, 3]);
        assert_eq!(join(4, &kept, 2), vec![(0, 3)]);
    }

    #[test]
    fn preserves_a_gap_one_larger_than_the_forgiveness() {
        let kept = BTreeSet::from([0, 3]);
        assert_eq!(join(4, &kept, 1), vec![(0, 0), (3, 3)]);
    }

    #[test]
    fn joins_nothing_when_every_slice_is_removed() {
        assert!(join(4, &BTreeSet::new(), 3).is_empty());
    }
}
