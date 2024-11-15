use crate::{HitData, Ray, Shape, WorldSpace, AABB};
use euclid::Point3D;
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug)]
pub(crate) struct BVHTree<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    aabb: AABB<Space>,
    root: Option<Node<Space>>,
    unbounded: Vec<Arc<dyn Shape<Space>>>,
}

impl<Space> BVHTree<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    pub(crate) fn new(shapes: &[Arc<dyn Shape<Space>>]) -> Self {
        info!(
            "Creating Bounded Volume Hierarchy for {} shapes",
            shapes.len()
        );
        let mut bounded = Vec::with_capacity(shapes.len());
        let mut unbounded = Vec::with_capacity(shapes.len());

        for shape in shapes.iter() {
            let aabb = shape.bounding_box();

            let shape = Arc::clone(shape);
            match aabb {
                Some(aabb) => bounded.push(Arc::new(BoundedShape { aabb, shape })),
                None => unbounded.push(shape),
            }
        }

        let aabb = bounding_box_for_shapes(&bounded);
        let root = (!shapes.is_empty()).then(|| {
            let (root, depth) = Node::new(bounded);
            info!("Created Bounded Volume Hierarchy with depth {}", depth);
            root
        });
        Self {
            aabb,
            unbounded,
            root,
        }
    }
}

impl BVHTree<WorldSpace> {
    pub(crate) fn intersects(&self, ray: Ray) -> Option<HitData> {
        vec![
            self.intersects_bounded_volume(ray),
            self.intersects_unbounded_volume(ray),
        ]
        .into_iter()
        .flatten()
        .min_by(|hit1, hit2| hit1.dist_to.partial_cmp(&hit2.dist_to).unwrap())
    }

    fn intersects_bounded_volume(&self, ray: Ray) -> Option<HitData> {
        let (tmin, tmax) = bounding_box_intersects(self.aabb, ray);
        if tmax < tmin || tmax <= 0.0 {
            None
        } else {
            self.root
                .as_ref()
                .and_then(|root| root.intersects(ray, tmin, tmax))
        }
    }

    fn intersects_unbounded_volume(&self, ray: Ray) -> Option<HitData> {
        self.unbounded
            .iter()
            .filter_map(|geom| geom.hit_by(&ray))
            .min_by(|hit1, hit2| hit1.dist_to.partial_cmp(&hit2.dist_to).unwrap())
    }
}

#[derive(Debug)]
enum Node<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    Parent(ParentNode<Space>),
    Leaf(LeafNode<Space>),
}

#[derive(Debug)]
struct ParentNode<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    axis: Axis,
    point: f64,
    left: Box<Node<Space>>,
    right: Box<Node<Space>>,
}

impl ParentNode<WorldSpace> {
    fn intersects(&self, ray: Ray, tmin: f64, tmax: f64) -> Option<HitData> {
        let rp: f64;
        let rd: f64;
        match self.axis {
            Axis::X => {
                rp = ray.point.x;
                rd = ray.dir.x;
            }
            Axis::Y => {
                rp = ray.point.y;
                rd = ray.dir.y;
            }
            Axis::Z => {
                rp = ray.point.z;
                rd = ray.dir.z;
            }
        };
        let tsplit = (self.point - rp) / rd;
        let left_first = (rp < self.point) || (rp == self.point && rd <= 0.0);

        let (first, second) = if left_first {
            (&self.left, &self.right)
        } else {
            (&self.right, &self.left)
        };
        if tsplit > tmax || tsplit <= 0.0 {
            first.intersects(ray, tmin, tmax)
        } else if tsplit < tmin {
            second.intersects(ray, tmin, tmax)
        } else {
            let h1 = first.intersects(ray, tmin, tsplit);

            if h1.is_some_and(|hit| hit.dist_to <= tsplit) {
                return h1;
            }

            let h1t = h1.map(|hit| hit.dist_to).unwrap_or(f64::MAX);

            let h2 = second.intersects(ray, tsplit, f64::min(tmax, h1t));
            let h2t = h2.map(|hit| hit.dist_to).unwrap_or(f64::MAX);

            if h1t < h2t {
                h1
            } else {
                h2
            }
        }
    }
}

#[derive(Debug)]
struct LeafNode<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    shapes: Vec<Arc<BoundedShape<Space>>>,
}

type PartitionedSegments<Space> = (Vec<Arc<BoundedShape<Space>>>, Vec<Arc<BoundedShape<Space>>>);

impl<Space> LeafNode<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    fn partition(&self, best: u64, best_axis: Axis, best_point: f64) -> PartitionedSegments<Space> {
        let mut left = Vec::with_capacity(best as usize);
        let mut right = Vec::with_capacity(best as usize);
        for shape in &self.shapes {
            let (l, r) = partition_bounding_box(best_axis, shape.aabb, best_point);
            if l {
                left.push(Arc::clone(shape));
            }
            if r {
                right.push(Arc::clone(shape));
            }
        }
        (left, right)
    }

    fn partition_score(&self, axis: Axis, axis_median: f64) -> u64 {
        let mut left = 0u64;
        let mut right = 0u64;
        for shape in &self.shapes {
            let (l, r) = partition_bounding_box(axis, shape.aabb, axis_median);
            if l {
                left += 1;
            }
            if r {
                right += 1;
            }
        }
        if left >= right {
            left
        } else {
            right
        }
    }
}

impl LeafNode<WorldSpace> {
    fn intersects(&self, ray: Ray) -> Option<HitData> {
        self.shapes
            .iter()
            .filter_map(|geom| geom.shape.hit_by(&ray))
            .min_by(|hit1, hit2| hit1.dist_to.partial_cmp(&hit2.dist_to).unwrap())
    }
}

impl<Space> Node<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    fn new(shapes: Vec<Arc<BoundedShape<Space>>>) -> (Self, usize) {
        let mut node = Self::Leaf(LeafNode { shapes });
        let depth = node.split();
        (node, depth + 1)
    }

    fn split(&mut self) -> usize {
        let leaf = match self {
            Self::Parent(_) => return 0,
            Self::Leaf(leaf) => leaf,
        };
        let shapes = &leaf.shapes;
        if shapes.len() < 8 {
            return 1;
        }

        let mut xs = Vec::with_capacity(shapes.len() * 2);
        let mut ys = Vec::with_capacity(shapes.len() * 2);
        let mut zs = Vec::with_capacity(shapes.len() * 2);
        for shape in shapes {
            xs.push(shape.aabb.min.x);
            xs.push(shape.aabb.max.x);
            ys.push(shape.aabb.min.y);
            ys.push(shape.aabb.max.y);
            zs.push(shape.aabb.min.z);
            zs.push(shape.aabb.max.z);
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        zs.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mx = median(&xs);
        let my = median(&ys);
        let mz = median(&zs);

        let mut best = (shapes.len() as f64 * 0.85) as u64;
        let mut best_axis: Option<Axis> = None;
        let mut best_point = 0.0;

        let sx = leaf.partition_score(Axis::X, mx);
        if sx < best {
            best = sx;
            best_axis = Some(Axis::X);
            best_point = mx;
        }
        let sy = leaf.partition_score(Axis::Y, my);
        if sy < best {
            best = sy;
            best_axis = Some(Axis::Y);
            best_point = my;
        }
        let sz = leaf.partition_score(Axis::Z, mz);
        if sz < best {
            best = sz;
            best_axis = Some(Axis::Z);
            best_point = mz;
        }
        if best_axis.is_none() {
            return 1;
        }
        let (l, r) = leaf.partition(best, best_axis.unwrap(), best_point);
        let (left, depth) = Node::new(l);
        let (right, _) = Node::new(r);
        *self = Self::Parent(ParentNode {
            axis: best_axis.unwrap(),
            point: best_point,
            left: Box::new(left),
            right: Box::new(right),
        });
        depth
    }
}

impl Node<WorldSpace> {
    fn intersects(&self, ray: Ray, tmin: f64, tmax: f64) -> Option<HitData> {
        match self {
            Self::Parent(parent_node) => parent_node.intersects(ray, tmin, tmax),
            Self::Leaf(leaf_node) => leaf_node.intersects(ray),
        }
    }
}

#[derive(Debug)]
struct BoundedShape<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    shape: Arc<dyn Shape<Space>>,
    aabb: AABB<Space>,
}

fn bounding_box_for_shapes<Space>(shapes: &[Arc<BoundedShape<Space>>]) -> AABB<Space>
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    let aabb = AABB::new(Point3D::splat(f64::MAX), Point3D::splat(f64::MIN));
    let bounding_boxes = shapes.iter().map(|shape| shape.aabb).collect::<Vec<_>>();

    bounding_boxes.into_par_iter().reduce(
        || aabb,
        |a, b| {
            let min = a.min.min(b.min);
            let max = a.max.max(b.max);

            AABB::new(min, max)
        },
    )
}

fn partition_bounding_box<Space>(axis: Axis, aabb: AABB<Space>, point: f64) -> (bool, bool)
where
    Space: Copy + Send + Sync + Sized + std::fmt::Debug + 'static,
{
    match axis {
        Axis::X => (aabb.min.x <= point, aabb.max.x >= point),
        Axis::Y => (aabb.min.y <= point, aabb.max.y >= point),
        Axis::Z => (aabb.min.z <= point, aabb.max.z >= point),
    }
}

fn bounding_box_intersects(aabb: AABB<WorldSpace>, ray: Ray) -> (f64, f64) {
    let v1 = (aabb.min - ray.point).component_div(ray.dir);
    let v2 = (aabb.max - ray.point).component_div(ray.dir);

    let ov1 = v1.min(v2);
    let ov2 = v1.max(v2);

    let t1 = f64::max(f64::max(ov1.x, ov1.y), ov1.z);
    let t2 = f64::min(f64::min(ov2.x, ov2.y), ov2.z);
    (t1, t2)
}

fn median(nums: &[f64]) -> f64 {
    let len = nums.len();
    match len {
        0 => 0.0,
        n if n % 2 == 1 => nums[len / 2],
        _ => {
            let a = nums[len / 2 - 1];
            let b = nums[len / 2 - 1];
            (a + b) / 2.0
        }
    }
}
