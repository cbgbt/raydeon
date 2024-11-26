use bvh::{BVHTree, Collidable};
use camera::{Observation, Perspective};
use collision::Continuous;
use euclid::{Point2D, Vector2D};
use material::Material;
use path::{LineSegment2D, SlicedSegment3D};
use rand::distributions::Distribution;
use rand::SeedableRng;
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::*;

const VERTICAL_HATCH_SCALING: f64 = 0.8;
const DIAGONAL_HATCH_SCALING: f64 = 0.46;
const HATCH_SPACING: f64 = PEN_PX_SIZE * 2.0;
const HATCH_PIXEL_CHOP_SIZE: f64 = PEN_PX_SIZE / 3.0;
const HATCHING_SLICE_FORGIVENESS: usize = 1;

#[derive(Debug)]
pub struct Scene<G, L> {
    geometry: G,
    lighting: L,
}

#[derive(Debug)]
pub struct SceneGeometry<P: PathMeta> {
    geometry: Vec<Arc<dyn Shape<WorldSpace, P>>>,
    bvh: BVHTree<WorldSpace, P>,
}

impl<P: PathMeta> SceneGeometry<P> {
    pub fn new() -> SceneGeometry<P> {
        Default::default()
    }

    pub fn with_geometry(mut self, geometry: Vec<Arc<dyn Shape<WorldSpace, P>>>) -> Self {
        let bvh = Self::create_bvh(&geometry);
        self.geometry = geometry;
        self.bvh = bvh;
        self
    }

    pub fn push_geometry(mut self, geometry: Arc<dyn Shape<WorldSpace, P>>) -> Self {
        self.geometry.push(geometry);
        self.bvh = Self::create_bvh(&self.geometry);
        self
    }

    pub fn concat_geometry(mut self, geometry: &[Arc<dyn Shape<WorldSpace, P>>]) -> Self {
        self.geometry.extend_from_slice(geometry);
        self.bvh = Self::create_bvh(&self.geometry);
        self
    }

    fn create_bvh(geometry: &[Arc<dyn Shape<WorldSpace, P>>]) -> BVHTree<WorldSpace, P> {
        let collision_geometry: Vec<_> = geometry
            .iter()
            .filter_map(|s| {
                s.collision_geometry().map(|collision_geom| {
                    collision_geom
                        .into_iter()
                        .map(|geom| (s.clone(), geom))
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
            .map(|(s, collision_geom)| Collidable::new(s, collision_geom))
            .collect();
        BVHTree::new(&collision_geometry)
    }
}

impl<P: PathMeta> Default for SceneGeometry<P> {
    fn default() -> Self {
        Self {
            geometry: Vec::new(),
            bvh: BVHTree::new(&[]),
        }
    }
}

impl<P: PathMeta, S: Shape<WorldSpace, P> + 'static> From<Vec<Arc<S>>> for SceneGeometry<P> {
    fn from(geometry: Vec<Arc<S>>) -> Self {
        SceneGeometry::new().with_geometry(
            geometry
                .into_iter()
                .map(|s| s as Arc<dyn Shape<WorldSpace, P>>)
                .collect::<Vec<_>>(),
        )
    }
}

impl<P: PathMeta> From<Vec<Arc<dyn Shape<WorldSpace, P>>>> for SceneGeometry<P> {
    fn from(geometry: Vec<Arc<dyn Shape<WorldSpace, P>>>) -> Self {
        SceneGeometry::new().with_geometry(geometry)
    }
}

#[derive(Debug, Default)]
pub struct SceneLighting {
    lights: Vec<Arc<dyn Light>>,
    ambient: f64,
}

impl SceneLighting {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_lights(mut self, lights: Vec<Arc<dyn Light>>) -> Self {
        self.lights = lights;
        self
    }

    pub fn push_light(mut self, light: Arc<dyn Light>) -> Self {
        self.lights.push(light);
        self
    }

    pub fn concat_lights(mut self, lights: &[Arc<dyn Light>]) -> Self {
        self.lights.extend_from_slice(lights);
        self
    }

    pub fn with_ambient_lighting(mut self, ambient: f64) -> Self {
        self.ambient = ambient;
        self
    }
}

impl<L: Light + 'static> From<Vec<Arc<L>>> for SceneLighting {
    fn from(lights: Vec<Arc<L>>) -> Self {
        let lights = lights
            .into_iter()
            .map(|l| l as Arc<dyn Light>)
            .collect::<Vec<_>>();
        SceneLighting::default().with_lights(lights)
    }
}

impl From<Vec<Arc<dyn Light>>> for SceneLighting {
    fn from(lights: Vec<Arc<dyn Light>>) -> Self {
        SceneLighting::default().with_lights(lights)
    }
}

impl Default for Scene<(), ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene<(), ()> {
    pub fn new() -> Scene<(), ()> {
        Scene {
            geometry: (),
            lighting: (),
        }
    }
}

impl<G, L> Scene<G, L> {
    pub fn with_geometry<P>(
        self,
        geometry: impl Into<SceneGeometry<P>>,
    ) -> Scene<SceneGeometry<P>, L>
    where
        P: PathMeta,
    {
        let Self { lighting, .. } = self;
        let geometry = geometry.into();
        Scene { geometry, lighting }
    }

    pub fn with_lighting(self, lighting: impl Into<SceneLighting>) -> Scene<G, SceneLighting> {
        let Self { geometry, .. } = self;
        let lighting = lighting.into();
        Scene { geometry, lighting }
    }
}

impl<L, P> Scene<SceneGeometry<P>, L>
where
    P: PathMeta,
    L: Send + Sync + 'static,
{
    pub fn attach_camera(&self, camera: Camera<Perspective, Observation>) -> SceneCamera<P, L> {
        SceneCamera::new(camera, self)
    }

    /// Find's the closest intersection point to geometry in the scene, if any
    pub(crate) fn intersects(&self, ray: Ray) -> Option<(HitData, Arc<dyn Shape<WorldSpace, P>>)> {
        self.geometry.bvh.intersects(ray)
    }

    /// Returns whether or not the given camera has a clear line of sight to a given point.
    fn visible(&self, from: WPoint3, point: WPoint3) -> bool {
        let v = from - point;
        let r = Ray::new(point, v.normalize());

        match self.intersects(r) {
            Some((hitdata, _)) => {
                let diff = (hitdata.dist_to - v.length()).abs();
                diff < 1.0e-1
            }
            None => true,
        }
    }
}

pub struct SceneCamera<'s, P, L>
where
    P: PathMeta,
{
    camera: Camera<Perspective, Observation>,
    scene: &'s Scene<SceneGeometry<P>, L>,
    seed: Option<u64>,
}

impl<'a, P, L> SceneCamera<'a, P, L>
where
    P: PathMeta,
    L: Send + Sync + 'static,
{
    pub fn new(
        camera: Camera<Perspective, Observation>,
        scene: &'a Scene<SceneGeometry<P>, L>,
    ) -> Self {
        SceneCamera {
            camera,
            scene,
            seed: None,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    fn clip_filter<M: PathMeta>(&self, path: &LineSegment3D<WorldSpace, M>) -> bool {
        self.scene
            .visible(self.camera.observation.eye, path.midpoint())
    }

    pub fn render(&self) -> Vec<LineSegment2D<CameraSpace>> {
        info!("Querying geometry for subpaths");
        let parent_paths: Vec<LineSegment3D<WorldSpace, P>> = self
            .scene
            .geometry
            .geometry
            .iter()
            .flat_map(|s| s.paths(&self.camera))
            .collect();

        info!(
            "Caching line segment chunks based on camera position, starting with {} segments",
            parent_paths.len()
        );

        let mut paths: Vec<SlicedSegment3D<WorldSpace, P>> = parent_paths
            .iter()
            .filter_map(|path| self.camera.chop_segment(path))
            .collect();

        let path_count: usize = paths
            .par_iter()
            .map(|subsegments| subsegments.num_subsegments())
            .sum();

        info!(
            "Done caching line segment chunks, created {} segment chunks",
            path_count
        );

        info!("Clipping occluded and distant segment chunks");

        let transformation: Transform3<WorldSpace, CameraSpace> =
            self.camera.camera_transformation();

        let paths: Vec<_> = paths
            .par_iter_mut()
            .flat_map(|path_group| {
                let to_remove: Vec<usize> = path_group
                    .subsegments()
                    .enumerate()
                    .filter_map(|(ndx, path)| {
                        let from_cam = path.midpoint() - self.camera.observation.eye;
                        let close_enough = from_cam.length() < self.camera.perspective.zfar;
                        let visible = close_enough && self.clip_filter(&path);
                        (!visible).then_some(ndx)
                    })
                    .collect();
                to_remove
                    .into_iter()
                    .for_each(|ndx| path_group.remove_subsegment(ndx));
                path_group.join_slices()
            })
            .filter_map(|path| path.transform(&transformation))
            .map(LineSegment3D::xy)
            .collect();

        info!("{} paths remain after clipping", paths.len());

        paths
    }
}

pub struct LitScene {
    pub geometry_paths: Vec<LineSegment2D<CameraSpace>>,
    pub hatch_paths: Vec<LineSegment2D<CameraSpace>>,
}

impl<'a> SceneCamera<'a, Material, SceneLighting> {
    pub fn render_with_lighting(&self) -> LitScene {
        let geometry_paths = self.render();
        let mut rng = match self.seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        info!("Generating vertical hatch lines from lighting.");
        let vert_lines = self.filter_hatch_lines_by(&self.vertical_hatch_lines(), |brightness| {
            let threshold: f64 = rand::distributions::Standard.sample(&mut rng);
            brightness > (threshold * VERTICAL_HATCH_SCALING)
        });
        info!("Generated {} vertical hatch lines.", vert_lines.len());

        info!("Generating diagonal hatch lines from lighting.");
        let diag_lines = self.filter_hatch_lines_by(&self.diagonal_hatch_lines(), |brightness| {
            let threshold: f64 = rand::distributions::Standard.sample(&mut rng);
            brightness > (threshold * DIAGONAL_HATCH_SCALING)
        });
        info!("Generated {} diagonal hatch lines.", diag_lines.len());

        let hatch_paths = [vert_lines, diag_lines].concat();

        // Compute visible lighting and do screen-space hatching

        LitScene {
            geometry_paths,
            hatch_paths,
        }
    }

    fn filter_hatch_lines_by(
        &self,
        segments: &[LineSegment2D<CameraSpace>],
        mut filter: impl FnMut(f64) -> bool,
    ) -> Vec<LineSegment2D<CameraSpace>> {
        let segments = segments
            .iter()
            .map(|segment| LineSegment3D::new(segment.p1.to_3d(), segment.p2.to_3d()))
            .collect::<Vec<_>>();
        let mut split_segments = segments
            .iter()
            .map(|segment| {
                let num_chops = (segment.length().ceil() / HATCH_PIXEL_CHOP_SIZE) as usize;
                SlicedSegment3D::new(num_chops, segment)
            })
            .collect::<Vec<_>>();

        let paths = split_segments
            .iter_mut()
            .flat_map(|path_group| {
                let to_remove = path_group
                    .subsegments()
                    .enumerate()
                    .filter_map(|(ndx, path)| {
                        let midpoint = path.midpoint();
                        let ray = self.camera.ray_for_px_coords(midpoint.x, midpoint.y);
                        let lighting = self.lighting_for_ray(ray);
                        (lighting.is_none() || filter(lighting.unwrap())).then_some(ndx)
                    })
                    .collect::<Vec<_>>();

                to_remove
                    .into_iter()
                    .for_each(|ndx| path_group.remove_subsegment(ndx));
                path_group.join_slices_with_forgiveness(HATCHING_SLICE_FORGIVENESS)
            })
            .map(|path| LineSegment2D::new(path.p1().to_2d(), path.p2().to_2d()))
            .collect::<Vec<_>>();

        paths
    }

    fn lighting_for_ray(&self, ray: Ray) -> Option<f64> {
        let lighting = self.scene.intersects(ray).and_then(|(hitpoint, shape)| {
            if hitpoint.dist_to > self.camera.perspective.zfar {
                return None;
            }
            Some(
                self.scene
                    .lighting
                    .lights
                    .iter()
                    .map(|light| light.compute_illumination(self.scene, hitpoint, &shape))
                    .sum::<f64>()
                    + self.scene.lighting.ambient,
            )
        });

        lighting
    }

    // https://smashingpencilsart.com/how-do-you-hatch-with-a-pen/
    fn vertical_hatch_lines(&self) -> Vec<LineSegment2D<CameraSpace>> {
        let initial_offset = HATCH_SPACING / 2.0;

        let mut segments = Vec::new();

        // vertical lines
        let mut x = initial_offset;
        while x < self.camera.perspective.width as f64 {
            let start = Point2::new(x, 0.0);
            let end = Point2::new(x, self.camera.perspective.height as f64);
            segments.push(LineSegment2D::new(start, end));
            x += HATCH_SPACING;
        }
        segments
    }

    fn diagonal_hatch_lines(&self) -> Vec<LineSegment2D<CameraSpace>> {
        let initial_offset = HATCH_SPACING / 2.0;

        let mut segments = Vec::new();

        // 60degree lines
        let hatch_dir = 120.0 * std::f64::consts::PI / 180.0;
        let hatch_dir: Vector2D<f64, CameraSpace> =
            Vec2::new(hatch_dir.cos(), hatch_dir.sin()).normalize();

        let coll_aabb = collision::Aabb2::new(
            (0.0, 0.0).into(),
            (
                self.camera.perspective.width as f64,
                self.camera.perspective.height as f64,
            )
                .into(),
        );
        let euclid_aabb = euclid::Box2D::new(
            (0.0, 0.0).into(),
            (
                self.camera.perspective.width as f64,
                self.camera.perspective.height as f64,
            )
                .into(),
        );

        let diagonal: Vector2D<f64, CameraSpace> = Vec2::new(
            self.camera.perspective.width as f64,
            self.camera.perspective.height as f64,
        )
        .normalize();
        let mut dist = initial_offset;
        let mut curr_point = diagonal * dist;
        while euclid_aabb.contains(curr_point.to_point()) {
            let start = curr_point;

            let cgstart = cgmath::Point2::from(start.to_array());
            let cgd1 = cgmath::Vector2::from(hatch_dir.to_array());
            let cgd2 = cgd1 * -1.0;
            let r1 = collision::Ray::new(cgstart, cgd1);
            let r2 = collision::Ray::new(cgstart, cgd2);

            let p1 = coll_aabb.intersection(&r1).unwrap();
            let p2 = coll_aabb.intersection(&r2).unwrap();

            segments.push(LineSegment2D::new(
                Point2D::new(p1.x, p1.y),
                Point2D::new(p2.x, p2.y),
            ));

            dist += HATCH_SPACING;
            curr_point = diagonal * dist;
        }

        segments
    }
}
