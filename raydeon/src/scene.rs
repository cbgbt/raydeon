use bon::Builder;
use bvh::{BVHTree, Collidable};
use collision::Continuous;
use euclid::{Point2D, Vector2D};
use path::{LineSegment2D, SlicedSegment3D};
use rand::distributions::Distribution;
use rand::SeedableRng;
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::*;

#[derive(Debug, Builder)]
#[builder(start_fn(name = new), finish_fn(name = construct))]
pub struct Scene {
    #[builder(into)]
    geometry: SceneGeometry,
    #[builder(into, default)]
    lighting: SceneLighting,
}

#[derive(Debug)]
pub struct SceneGeometry {
    geometry: Vec<Arc<dyn Shape>>,
    bvh: BVHTree,
}

impl SceneGeometry {
    pub fn new() -> SceneGeometry {
        Default::default()
    }

    pub fn with_geometry(mut self, geometry: Vec<Arc<dyn Shape>>) -> Self {
        let bvh = Self::create_bvh(&geometry);
        self.geometry = geometry;
        self.bvh = bvh;
        self
    }

    pub fn push_geometry(mut self, geometry: Arc<dyn Shape>) -> Self {
        self.geometry.push(geometry);
        self.bvh = Self::create_bvh(&self.geometry);
        self
    }

    pub fn concat_geometry(mut self, geometry: &[Arc<dyn Shape>]) -> Self {
        self.geometry.extend_from_slice(geometry);
        self.bvh = Self::create_bvh(&self.geometry);
        self
    }

    fn create_bvh(geometry: &[Arc<dyn Shape>]) -> BVHTree {
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

impl Default for SceneGeometry {
    fn default() -> Self {
        Self {
            geometry: Vec::new(),
            bvh: BVHTree::new(&[]),
        }
    }
}

impl<S: Shape + 'static> From<Vec<Arc<S>>> for SceneGeometry {
    fn from(geometry: Vec<Arc<S>>) -> Self {
        SceneGeometry::new().with_geometry(
            geometry
                .into_iter()
                .map(|s| s as Arc<dyn Shape>)
                .collect::<Vec<_>>(),
        )
    }
}

impl From<Vec<Arc<dyn Shape>>> for SceneGeometry {
    fn from(geometry: Vec<Arc<dyn Shape>>) -> Self {
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

impl Scene {
    pub fn attach_camera(&self, camera: Camera) -> SceneCamera {
        SceneCamera::new(camera, self)
    }

    /// Find's the closest intersection point to geometry in the scene, if any
    pub(crate) fn intersects(&self, ray: Ray) -> Option<(HitData, Arc<dyn Shape>)> {
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

#[derive(Debug)]
pub struct SceneCamera<'s> {
    camera: Camera,
    scene: &'s Scene,
    seed: Option<u64>,
}

impl<'a> SceneCamera<'a> {
    pub fn new(camera: Camera, scene: &'a Scene) -> Self {
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

    fn clip_filter(&self, path: &LineSegment3D<WorldSpace>) -> bool {
        self.scene
            .visible(self.camera.observation.eye, path.midpoint())
    }

    pub fn render(&self) -> Vec<LineSegment2D<CameraSpace>> {
        info!("Querying geometry for subpaths");
        let parent_paths: Vec<LineSegment3D<WorldSpace>> = self
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

        let mut paths: Vec<SlicedSegment3D<WorldSpace>> = parent_paths
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

impl<'a> SceneCamera<'a> {
    pub fn render_with_lighting(&self) -> LitScene {
        let geometry_paths = self.render();
        let mut rng = match self.seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        info!("Generating vertical hatch lines from lighting.");
        let vert_lines = self.filter_hatch_lines_by(&self.vertical_hatch_lines(), |brightness| {
            let threshold: f64 = rand::distributions::Standard.sample(&mut rng);
            brightness > (threshold * self.camera.render_options.vert_hatch_brightness_scaling)
        });
        info!("Generated {} vertical hatch lines.", vert_lines.len());

        info!("Generating diagonal hatch lines from lighting.");
        let diag_lines = self.filter_hatch_lines_by(&self.diagonal_hatch_lines(), |brightness| {
            let threshold: f64 = rand::distributions::Standard.sample(&mut rng);
            brightness > (threshold * self.camera.render_options.diag_hatch_brightness_scaling)
        });
        info!("Generated {} diagonal hatch lines.", diag_lines.len());

        let hatch_paths = [vert_lines, diag_lines].concat();

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
            .map(|segment| {
                LineSegment3D::new()
                    .p1(segment.p1.to_3d())
                    .p2(segment.p2.to_3d())
                    .build()
            })
            .collect::<Vec<_>>();
        let mut split_segments = segments
            .iter()
            .map(|segment| {
                let num_chops = (segment.length().ceil()
                    / self.camera.render_options.hatch_pixel_chop_factor)
                    as usize;
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
                path_group.join_slices_with_forgiveness(
                    self.camera.render_options.hatch_slice_forgiveness,
                )
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
        let initial_offset = self.camera.render_options.hatch_pixel_spacing / 2.0;

        let mut segments = Vec::new();

        // vertical lines
        let mut x = initial_offset;
        while x < self.camera.perspective.width as f64 {
            let start = Point2::new(x, 0.0);
            let end = Point2::new(x, self.camera.perspective.height as f64);
            segments.push(LineSegment2D::new(start, end));
            x += self.camera.render_options.hatch_pixel_spacing;
        }
        segments
    }

    fn diagonal_hatch_lines(&self) -> Vec<LineSegment2D<CameraSpace>> {
        let initial_offset = self.camera.render_options.hatch_pixel_spacing / 2.0;

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

            dist += self.camera.render_options.hatch_pixel_spacing;
            curr_point = diagonal * dist;
        }

        segments
    }
}
