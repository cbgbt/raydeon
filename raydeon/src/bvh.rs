use crate::ray::HitShape;
use crate::{CollisionGeometry, DrawableShape, Ray, WorldSpace, AABB3};
use euclid::Point3D;
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone)]
pub(crate) struct Collidable {
    shape: DrawableShape,
    collision: Arc<dyn CollisionGeometry>,
}

impl Collidable {
    pub(crate) fn new(shape: DrawableShape, collision: Arc<dyn CollisionGeometry>) -> Self {
        Self { shape, collision }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug)]
pub(crate) struct BVHTree {
    aabb: AABB3<WorldSpace>,
    root: Option<Node>,
    unbounded: Vec<Collidable>,
}

impl BVHTree {
    pub(crate) fn new(collidables: &[Collidable]) -> Self {
        info!(
            "Creating Bounded Volume Hierarchy for {} shapes",
            collidables.len()
        );
        let mut bounded = Vec::with_capacity(collidables.len());
        let mut unbounded = Vec::with_capacity(collidables.len());

        for collidable in collidables.iter() {
            let aabb = collidable.collision.bounding_box();

            let collidable = collidable.clone();
            match aabb {
                Some(aabb) => bounded.push(Arc::new(BoundedShape { aabb, collidable })),
                None => unbounded.push(collidable),
            }
        }

        let aabb = bounding_box_for_shapes(&bounded);
        let root = (!collidables.is_empty()).then(|| {
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

impl BVHTree {
    pub(crate) fn intersects(&self, ray: Ray) -> Option<HitShape> {
        vec![
            self.intersects_bounded_volume(ray),
            self.intersects_unbounded_volume(ray),
        ]
        .into_iter()
        .flatten()
        .min_by(|hit1, hit2| hit1.hit_data.dist_to.total_cmp(&hit2.hit_data.dist_to))
    }

    fn intersects_bounded_volume(&self, ray: Ray) -> Option<HitShape> {
        let (tmin, tmax) = bounding_box_intersects(self.aabb, ray);
        if tmax < tmin || tmax <= 0.0 {
            None
        } else {
            self.root
                .as_ref()
                .and_then(|root| root.intersects(ray, tmin, tmax))
        }
    }

    fn intersects_unbounded_volume(&self, ray: Ray) -> Option<HitShape> {
        self.unbounded
            .iter()
            .filter_map(|collidable| {
                collidable
                    .collision
                    .hit_by(&ray)
                    .map(|hit_point| HitShape::new(hit_point, &collidable.shape))
            })
            .min_by(|hit1, hit2| hit1.hit_data.dist_to.total_cmp(&hit2.hit_data.dist_to))
    }
}

#[derive(Debug)]
enum Node {
    Parent(ParentNode),
    Leaf(LeafNode),
}

#[derive(Debug)]
struct ParentNode {
    axis: Axis,
    point: f64,
    left: Box<Node>,
    right: Box<Node>,
}

impl ParentNode {
    fn intersects(&self, ray: Ray, tmin: f64, tmax: f64) -> Option<HitShape> {
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

            if h1
                .as_ref()
                .is_some_and(|hit| hit.hit_data.dist_to <= tsplit)
            {
                return h1;
            }

            let h1t = h1
                .as_ref()
                .map(|hit| hit.hit_data.dist_to)
                .unwrap_or(f64::MAX);

            let h2 = second.intersects(ray, tsplit, f64::min(tmax, h1t));
            let h2t = h2
                .as_ref()
                .map(|hit| hit.hit_data.dist_to)
                .unwrap_or(f64::MAX);

            if h1t < h2t {
                h1
            } else {
                h2
            }
        }
    }
}

#[derive(Debug)]
struct LeafNode {
    shapes: Vec<Arc<BoundedShape>>,
}

type PartitionedSegments = (Vec<Arc<BoundedShape>>, Vec<Arc<BoundedShape>>);

impl LeafNode {
    fn partition(&self, best: u64, best_axis: Axis, best_point: f64) -> PartitionedSegments {
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

impl LeafNode {
    fn intersects(&self, ray: Ray) -> Option<HitShape> {
        self.shapes
            .iter()
            .filter_map(|shape| {
                shape
                    .collidable
                    .collision
                    .hit_by(&ray)
                    .map(|hitpoint| HitShape::new(hitpoint, &shape.collidable.shape))
            })
            .min_by(|hit1, hit2| hit1.hit_data.dist_to.total_cmp(&hit2.hit_data.dist_to))
    }
}

impl Node {
    fn new(shapes: Vec<Arc<BoundedShape>>) -> (Self, usize) {
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
        xs.sort_by(f64::total_cmp);
        ys.sort_by(f64::total_cmp);
        zs.sort_by(f64::total_cmp);

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

impl Node {
    fn intersects(&self, ray: Ray, tmin: f64, tmax: f64) -> Option<HitShape> {
        match self {
            Self::Parent(parent_node) => parent_node.intersects(ray, tmin, tmax),
            Self::Leaf(leaf_node) => leaf_node.intersects(ray),
        }
    }
}

#[derive(Debug)]
struct BoundedShape {
    collidable: Collidable,
    aabb: AABB3<WorldSpace>,
}

fn bounding_box_for_shapes(shapes: &[Arc<BoundedShape>]) -> AABB3<WorldSpace> {
    let aabb = AABB3::new(Point3D::splat(f64::MAX), Point3D::splat(f64::MIN));
    let bounding_boxes = shapes.iter().map(|shape| shape.aabb).collect::<Vec<_>>();

    bounding_boxes.into_par_iter().reduce(
        || aabb,
        |a, b| {
            let min = a.min.min(b.min);
            let max = a.max.max(b.max);

            AABB3::new(min, max)
        },
    )
}

fn partition_bounding_box(axis: Axis, aabb: AABB3<WorldSpace>, point: f64) -> (bool, bool) {
    match axis {
        Axis::X => (aabb.min.x <= point, aabb.max.x >= point),
        Axis::Y => (aabb.min.y <= point, aabb.max.y >= point),
        Axis::Z => (aabb.min.z <= point, aabb.max.z >= point),
    }
}

fn bounding_box_intersects(aabb: AABB3<WorldSpace>, ray: Ray) -> (f64, f64) {
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
            let b = nums[len / 2];
            (a + b) / 2.0
        }
    }
}

/// The hierarchy is an acceleration structure, so the only thing worth
/// asserting about it is that it answers exactly what testing every shape
/// would.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::AxisAlignedCuboid;
    use crate::{DrawableShape, Shape, WPoint3, WVec3};
    use proptest::prelude::*;

    /// A cuboid as `(x, y, z, size)`.
    type CuboidSpec = (f64, f64, f64, f64);

    fn collidables(specs: &[CuboidSpec]) -> Vec<Collidable> {
        specs
            .iter()
            .map(|&(x, y, z, size)| {
                let cuboid = AxisAlignedCuboid::new()
                    .min((x, y, z))
                    .max((x + size, y + size, z + size))
                    .build();
                let geometry = Arc::new(cuboid) as Arc<dyn Shape>;
                let collision = geometry.collision_geometry().unwrap().remove(0);
                let shape = DrawableShape::new().geometry(geometry).build();
                Collidable::new(shape, collision)
            })
            .collect()
    }

    fn nearest_hit_by_brute_force(collidables: &[Collidable], ray: Ray) -> Option<f64> {
        collidables
            .iter()
            .filter_map(|collidable| collidable.collision.hit_by(&ray))
            .map(|hit| hit.dist_to)
            .min_by(f64::total_cmp)
    }

    fn cuboid_strategy() -> impl Strategy<Value = CuboidSpec> {
        (-10.0..10.0f64, -10.0..10.0f64, -10.0..10.0f64, 0.5..3.0f64)
    }

    /// Rays with an axis-aligned direction component sit exactly on a split
    /// plane's degenerate case; they are not what this test is about.
    fn ray_strategy() -> impl Strategy<Value = Ray> {
        let component = (-1.0..1.0f64).prop_map(|v| {
            if v.abs() < 0.05 {
                0.05_f64.copysign(v)
            } else {
                v
            }
        });
        (
            (-20.0..20.0f64, -20.0..20.0f64, -20.0..20.0f64),
            (component.clone(), component.clone(), component),
        )
            .prop_map(|((px, py, pz), (dx, dy, dz))| {
                Ray::normalize_new(WPoint3::new(px, py, pz), WVec3::new(dx, dy, dz))
            })
    }

    proptest! {
        #[test]
        fn matches_brute_force_hits(
            specs in proptest::collection::vec(cuboid_strategy(), 8..24),
            ray in ray_strategy(),
        ) {
            let collidables = collidables(&specs);
            let tree = BVHTree::new(&collidables);

            let from_tree = tree.intersects(ray).map(|hit| hit.hit_data.dist_to);
            let brute_force = nearest_hit_by_brute_force(&collidables, ray);

            match (from_tree, brute_force) {
                (Some(tree_dist), Some(brute_dist)) => {
                    prop_assert!(
                        (tree_dist - brute_dist).abs() < 1e-9,
                        "hierarchy found a hit at {} where brute force found {}",
                        tree_dist,
                        brute_dist
                    );
                }
                (tree_hit, brute_hit) => prop_assert_eq!(tree_hit, brute_hit),
            }
        }
    }
}
