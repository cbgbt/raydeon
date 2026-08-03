use bon::Builder;
use bvh::{BVHTree, Collidable};
use collision::Continuous;
use euclid::{Point2D, Vector2D, Vector3D};
use path::{LineSegment2D, SlicedSegment3D};
use rand::distributions::Distribution;
use rand::SeedableRng;
use ray::HitShape;
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::*;

#[derive(Debug, Builder)]
#[builder(start_fn(name = new))]
pub struct Scene {
    #[builder(into)]
    geometry: SceneGeometry,
    #[builder(into, default)]
    lighting: SceneLighting,
}

#[derive(Debug)]
pub struct SceneGeometry {
    geometry: Vec<DrawableShape>,
    bvh: BVHTree,
}

impl SceneGeometry {
    pub fn new() -> SceneGeometry {
        Default::default()
    }

    pub fn with_geometry(mut self, geometry: Vec<DrawableShape>) -> Self {
        let bvh = Self::create_bvh(&geometry);
        self.geometry = geometry;
        self.bvh = bvh;
        self
    }

    pub fn push_geometry(mut self, geometry: DrawableShape) -> Self {
        self.geometry.push(geometry);
        self.bvh = Self::create_bvh(&self.geometry);
        self
    }

    pub fn concat_geometry(mut self, geometry: &[DrawableShape]) -> Self {
        self.geometry.extend_from_slice(geometry);
        self.bvh = Self::create_bvh(&self.geometry);
        self
    }

    fn create_bvh(geometry: &[DrawableShape]) -> BVHTree {
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
                .map(|s| DrawableShape::new().geometry(s as Arc<dyn Shape>).build())
                .collect::<Vec<_>>(),
        )
    }
}

impl From<Vec<Arc<dyn Shape>>> for SceneGeometry {
    fn from(shapes: Vec<Arc<dyn Shape>>) -> Self {
        SceneGeometry::new().with_geometry(
            shapes
                .into_iter()
                .map(|shapes| DrawableShape::new().geometry(shapes).build())
                .collect(),
        )
    }
}

impl From<Vec<DrawableShape>> for SceneGeometry {
    fn from(geometry: Vec<DrawableShape>) -> Self {
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
    pub(crate) fn intersects(&self, ray: Ray) -> Option<HitShape> {
        self.geometry.bvh.intersects(ray)
    }

    /// Computes the total illumination arriving at a surface point from the
    /// scene's lights, including ambient light.
    ///
    /// Callers which already know the surface geometry can construct the
    /// [`HitShape`] directly rather than casting a ray, which is useful for
    /// sampling illumination along surface-space hatch lines.
    ///
    /// `eye` is the viewpoint the illumination is seen from; specular
    /// highlights depend on it.
    pub fn illumination_for_hit<'s>(&'s self, hit: HitShape<'s>, eye: WPoint3) -> f64 {
        self.lighting
            .lights
            .iter()
            .map(|light| light.compute_illumination(self, hit, eye))
            .sum::<f64>()
            + self.lighting.ambient
    }

    /// Returns whether or not the given camera has a clear line of sight to a given point.
    fn visible(&self, from: WPoint3, point: WPoint3) -> bool {
        let v = from - point;
        let r = Ray::new(point, v.normalize());

        match self.intersects(r) {
            Some(hit_shape) => {
                let diff = (hit_shape.hit_data.dist_to - v.length()).abs();
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

impl<'s> SceneCamera<'s> {
    pub fn new(camera: Camera, scene: &'s Scene) -> Self {
        SceneCamera {
            camera,
            scene,
            seed: None,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> SceneCamera<'s> {
        self.seed = Some(seed);
        self
    }

    pub fn adjust_yaw(&mut self, yaw: euclid::Angle<f64>) {
        self.camera.adjust_yaw(yaw);
    }

    pub fn adjust_pitch(&mut self, pitch: euclid::Angle<f64>) {
        self.camera.adjust_pitch(pitch);
    }

    pub fn adjust_roll(&mut self, roll: euclid::Angle<f64>) {
        self.camera.adjust_roll(roll);
    }

    pub fn translate(&mut self, trans: impl Into<Vector3D<f64, ()>>) {
        self.camera.translate(trans);
    }

    fn clip_filter(&self, path: &LineSegment3D<WorldSpace>) -> bool {
        self.scene
            .visible(self.camera.observation.eye(), path.midpoint())
    }

    pub fn render(&self) -> Rendering {
        info!("Querying geometry for subpaths");

        // Each shape is processed on its own so that every stroke can carry the
        // pen of the material which drew it.
        let strokes = self
            .scene
            .geometry
            .geometry
            .par_iter()
            .flat_map(|shape| {
                let pen = shape.material().map(|mat| mat.pen).unwrap_or_default();
                let paths = shape.paths(&self.camera);
                self.clip_and_project(&paths)
                    .into_iter()
                    .map(|segment| Stroke {
                        p1: segment.p1,
                        p2: segment.p2,
                        pen,
                        kind: StrokeKind::Outline,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        Rendering::new(strokes)
    }

    /// Clips the given world-space segments against scene geometry, keeping
    /// only portions visible to the camera, and projects them to camera space.
    ///
    /// Segments lying on a surface of scene geometry survive their own
    /// surface's occlusion check, so this can render caller-generated
    /// surface detail (such as hatch lines) with correct hidden-line removal.
    pub fn clip_and_project(
        &self,
        parent_paths: &[LineSegment3D<WorldSpace>],
    ) -> Vec<LineSegment2D<CameraSpace>> {
        info!(
            "Caching line segment chunks based on camera position, starting with {} segments",
            parent_paths.len()
        );

        let mut paths: Vec<SlicedSegment3D<WorldSpace>> = parent_paths
            .iter()
            .filter_map(|segment| self.camera.chop_segment(segment))
            .collect();

        let path_count: usize = paths
            .iter()
            .map(|subsegments| subsegments.num_subsegments())
            .sum();

        info!(
            "Done caching line segment chunks, created {} segment chunks",
            path_count
        );

        info!("Clipping occluded and distant segment chunks");

        let transformation: Transform3<WorldSpace, CameraSpace> =
            self.camera.camera_transformation();

        let paths: Vec<LineSegment2D<CameraSpace>> = paths
            .par_iter_mut()
            .flat_map(|path_group| {
                let to_remove: Vec<usize> = path_group
                    .subsegments()
                    .enumerate()
                    .filter_map(|(ndx, path)| {
                        let from_cam = path.midpoint() - self.camera.observation.eye();
                        let close_enough = from_cam.length() < self.camera.perspective.zfar();
                        let visible = close_enough && self.clip_filter(&path);
                        (!visible).then_some(ndx)
                    })
                    .collect();
                to_remove
                    .into_iter()
                    .for_each(|ndx| path_group.remove_subsegment(ndx));
                path_group.join_slices()
            })
            .filter_map(|path| path.transform(&transformation).map(LineSegment3D::xy))
            .collect();

        info!("{} paths remain after clipping", paths.len());

        paths
    }

    pub fn render_with_lighting(&self) -> Rendering {
        let geometry_render = self.render();
        let mut rng = match self.seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        info!("Generating vertical hatch lines from lighting.");
        let vert_lines = self
            .filter_hatch_lines_by(&self.vertical_hatch_lines(), |brightness| {
                let threshold: f64 = rand::distributions::Standard.sample(&mut rng);
                brightness > (threshold * self.camera.render_options.vert_hatch_brightness_scaling)
            })
            .into_iter()
            .map(screen_hatch_stroke)
            .collect::<Vec<_>>();

        info!("Generated {} vertical hatch lines.", vert_lines.len());

        info!("Generating diagonal hatch lines from lighting.");
        let diag_lines = self
            .filter_hatch_lines_by(&self.diagonal_hatch_lines(), |brightness| {
                let threshold: f64 = rand::distributions::Standard.sample(&mut rng);
                brightness > (threshold * self.camera.render_options.diag_hatch_brightness_scaling)
            })
            .into_iter()
            .map(screen_hatch_stroke)
            .collect::<Vec<_>>();
        info!("Generated {} diagonal hatch lines.", diag_lines.len());

        Rendering::new([geometry_render.strokes(), &vert_lines, &diag_lines].concat())
    }

    fn filter_hatch_lines_by(
        &self,
        segments: &[LineSegment2D<CameraSpace>],
        mut filter: impl FnMut(f64) -> bool,
    ) -> Vec<LineSegment2D<CameraSpace>> {
        let segments = segments
            .iter()
            .map(LineSegment2D::to_3d)
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
            .map(LineSegment3D::xy)
            .collect::<Vec<_>>();

        paths
    }

    fn lighting_for_ray(&self, ray: Ray) -> Option<f64> {
        let hit_shape = self.scene.intersects(ray)?;
        (hit_shape.hit_data.dist_to <= self.camera.perspective.zfar()).then(|| {
            self.scene
                .illumination_for_hit(hit_shape, self.camera.observation.eye())
        })
    }

    // https://smashingpencilsart.com/how-do-you-hatch-with-a-pen/
    fn vertical_hatch_lines(&self) -> Vec<LineSegment2D<CameraSpace>> {
        let initial_offset = self.camera.render_options.hatch_pixel_spacing / 2.0;

        let mut segments = Vec::new();

        // vertical lines
        let mut x = initial_offset;
        while x < self.camera.perspective.width() as f64 {
            let start = Point2::new(x, 0.0);
            let end = Point2::new(x, self.camera.perspective.height() as f64);
            segments.push(LineSegment2D::new_segment(start, end));
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
                self.camera.perspective.width() as f64,
                self.camera.perspective.height() as f64,
            )
                .into(),
        );
        let euclid_aabb = euclid::Box2D::new(
            (0.0, 0.0).into(),
            (
                self.camera.perspective.width() as f64,
                self.camera.perspective.height() as f64,
            )
                .into(),
        );

        let diagonal: Vector2D<f64, CameraSpace> = Vec2::new(
            self.camera.perspective.width() as f64,
            self.camera.perspective.height() as f64,
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

            segments.push(LineSegment2D::new_segment(
                Point2D::new(p1.x, p1.y),
                Point2D::new(p2.x, p2.y),
            ));

            dist += self.camera.render_options.hatch_pixel_spacing;
            curr_point = diagonal * dist;
        }

        segments
    }
}

/// Screen-space hatching shades the whole image rather than any one shape, so
/// its strokes plot with the default pen.
fn screen_hatch_stroke(segment: LineSegment2D<CameraSpace>) -> Stroke {
    Stroke {
        p1: segment.p1,
        p2: segment.p2,
        pen: PenId::default(),
        kind: StrokeKind::Hatch,
    }
}
