use bon::Builder;
use bvh::{BVHTree, Collidable};
use euclid::Vector2D;
use hatch::jitter::{jitter01, screen_seed};
use nutype::nutype;
use path::{LineSegment2D, SlicedSegment3D};
use ray::HitShape;
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::*;

/// The illumination at which a surface reads as fully white, and above which
/// hatching stops shading it. Tone is `(illumination / tone_white)` clamped
/// to `[0, 1]`, so a scene with brighter lights needs a larger value.
#[nutype(
    validate(finite, greater = 0.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct ToneWhite(f64);

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

#[derive(Debug)]
pub struct SceneLighting {
    lights: Vec<Arc<dyn Light>>,
    ambient: f64,
    tone_white: ToneWhite,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            lights: Vec::new(),
            ambient: 0.0,
            tone_white: ToneWhite::try_new(1.0).expect("one is a valid tone normalization"),
        }
    }
}

impl SceneLighting {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_lights(mut self, lights: Vec<Arc<dyn Light>>) -> Self {
        self.lights = lights;
        self
    }

    pub fn with_ambient_lighting(mut self, ambient: f64) -> Self {
        self.ambient = ambient;
        self
    }

    /// Sets the illumination which hatching reads as fully white.
    pub fn with_tone_white(mut self, tone_white: ToneWhite) -> Self {
        self.tone_white = tone_white;
        self
    }

    pub fn tone_white(&self) -> ToneWhite {
        self.tone_white
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
    pub fn attach_camera(&self, camera: Camera) -> SceneCamera<'_> {
        SceneCamera::new(camera, self)
    }

    /// Find's the closest intersection point to geometry in the scene, if any
    pub(crate) fn intersects(&self, ray: Ray) -> Option<HitShape<'_>> {
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

    /// The perceived brightness at a surface point, normalized to `[0, 1]`
    /// by the scene's `tone_white`: 0 is as dark as hatching shades, 1 is
    /// paper white.
    pub(crate) fn tone_for_hit(&self, hit: HitShape, eye: WPoint3) -> f64 {
        (self.illumination_for_hit(hit, eye) / self.lighting.tone_white.into_inner())
            .clamp(0.0, 1.0)
    }

    /// Returns whether or not the given camera has a clear line of sight to a given point.
    ///
    /// A point is occluded iff the nearest hit along the ray toward the eye
    /// lies strictly BETWEEN the point and the eye; a hit at or beyond the
    /// eye (e.g. geometry behind the camera) never occludes (invariant 21).
    fn visible(&self, from: WPoint3, point: WPoint3) -> bool {
        let v = from - point;
        let r = Ray::new(point, v.normalize());

        match self.intersects(r) {
            Some(hit_shape) => hit_shape.hit_data.dist_to >= v.length() - OCCLUSION_TOL,
            None => true,
        }
    }
}

/// Tolerance on the eye-distance comparison in `Scene::visible`: a hit within
/// this margin of the eye is treated as "at the eye", not an occluder.
const OCCLUSION_TOL: f64 = 1.0e-1;

#[derive(Debug)]
pub struct SceneCamera<'s> {
    camera: Camera,
    scene: &'s Scene,
    /// Salts the position-hashed noise which scatters hatching. The same
    /// scene, camera and seed always draw the same strokes.
    seed: u64,
}

impl<'s> SceneCamera<'s> {
    pub fn new(camera: Camera, scene: &'s Scene) -> Self {
        SceneCamera {
            camera,
            scene,
            seed: 0,
        }
    }

    /// Redraws the scene's hatching with a different scatter.
    pub fn with_seed(mut self, seed: u64) -> SceneCamera<'s> {
        self.seed = seed;
        self
    }

    pub(crate) fn scene(&self) -> &'s Scene {
        self.scene
    }

    pub(crate) fn eye(&self) -> WPoint3 {
        self.camera.observation.eye()
    }

    pub(crate) fn seed(&self) -> u64 {
        self.seed
    }

    fn clip_filter(&self, path: &LineSegment3D<WorldSpace>) -> bool {
        self.scene
            .visible(self.camera.observation.eye(), path.midpoint())
    }

    /// Draws the scene: the outlines of every shape, plus the hatching any
    /// material asks for, hidden-line removed and projected to the page.
    pub fn render(&self) -> Rendering {
        info!("Querying geometry for subpaths");

        // Each shape is drawn on its own so that every stroke can carry the
        // pen of the material which drew it.
        let strokes = self
            .scene
            .geometry
            .geometry
            .par_iter()
            .flat_map(|shape| {
                let material = shape.material();
                let pen = material.map(|mat| mat.pen).unwrap_or_default();

                let outlines =
                    self.strokes_from(&shape.paths(&self.camera), pen, StrokeKind::Outline);
                let Some(style) = material.and_then(|mat| mat.hatch.as_ref()) else {
                    return outlines;
                };

                let hatching = hatch::hatch_shape(self, shape, style);
                [
                    outlines,
                    self.strokes_from(&hatching, pen, StrokeKind::Hatch),
                ]
                .concat()
            })
            .collect();

        Rendering::new(strokes)
    }

    /// Clips world-space segments against the scene and turns what survives
    /// into strokes of the given pen.
    fn strokes_from(
        &self,
        segments: &[LineSegment3D<WorldSpace>],
        pen: PenId,
        kind: StrokeKind,
    ) -> Vec<Stroke> {
        self.clip_and_project(segments)
            .into_iter()
            .map(|segment| Stroke {
                p1: segment.p1,
                p2: segment.p2,
                pen,
                kind,
            })
            .collect()
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

    /// Draws the scene and shades the whole image with screen-space hatching:
    /// vertical and diagonal rules thinned out by how brightly each part of
    /// the image is lit.
    pub fn render_with_screen_hatching(&self) -> Rendering {
        let geometry_render = self.render();

        info!("Generating vertical hatch lines from lighting.");
        let vert_scaling = self.camera.render_options.vert_hatch_brightness_scaling;
        let vert_lines = self
            .filter_hatch_lines_by(&self.vertical_hatch_lines(), |brightness, at| {
                brightness > self.dither(at) * vert_scaling
            })
            .into_iter()
            .map(screen_hatch_stroke)
            .collect::<Vec<_>>();

        info!("Generated {} vertical hatch lines.", vert_lines.len());

        info!("Generating diagonal hatch lines from lighting.");
        let diag_scaling = self.camera.render_options.diag_hatch_brightness_scaling;
        let diag_lines = self
            .filter_hatch_lines_by(&self.diagonal_hatch_lines(), |brightness, at| {
                brightness > self.dither(at) * diag_scaling
            })
            .into_iter()
            .map(screen_hatch_stroke)
            .collect::<Vec<_>>();
        info!("Generated {} diagonal hatch lines.", diag_lines.len());

        Rendering::new([geometry_render.strokes(), &vert_lines, &diag_lines].concat())
    }

    /// The dither value a screen-space sample is compared against, hashed
    /// from where the sample sits on the page so that the same render always
    /// keeps the same chops.
    fn dither(&self, at: Point2<CameraSpace>) -> f64 {
        jitter01(screen_seed(at, self.seed))
    }

    fn filter_hatch_lines_by(
        &self,
        segments: &[LineSegment2D<CameraSpace>],
        filter: impl Fn(f64, Point2<CameraSpace>) -> bool + Sync,
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
            .par_iter_mut()
            .flat_map(|path_group| {
                let to_remove = path_group
                    .subsegments()
                    .enumerate()
                    .filter_map(|(ndx, path)| {
                        let midpoint = path.midpoint();
                        let ray = self.camera.ray_for_px_coords(midpoint.x, midpoint.y);
                        let lighting = self.lighting_for_ray(ray);
                        // A chop over unlit background, or over a surface
                        // bright enough to pass the filter, is not drawn.
                        let drop = match lighting {
                            Some(brightness) => filter(brightness, midpoint.xy()),
                            None => true,
                        };
                        drop.then_some(ndx)
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

        let page = euclid::Box2D::new(
            Point2::new(0.0, 0.0),
            Point2::new(
                self.camera.perspective.width() as f64,
                self.camera.perspective.height() as f64,
            ),
        );

        let diagonal: Vector2D<f64, CameraSpace> = Vec2::new(
            self.camera.perspective.width() as f64,
            self.camera.perspective.height() as f64,
        )
        .normalize();
        let mut dist = initial_offset;
        let mut curr_point = diagonal * dist;
        while page.contains(curr_point.to_point()) {
            if let Some((p1, p2)) = clip_line_to_box(curr_point.to_point(), hatch_dir, &page) {
                segments.push(LineSegment2D::new_segment(p1, p2));
            }

            dist += self.camera.render_options.hatch_pixel_spacing;
            curr_point = diagonal * dist;
        }

        segments
    }
}

/// Clips the infinite line through `origin` along `dir` to `page`, returning
/// the two boundary points, or `None` when the line misses the box entirely.
///
/// Liang-Barsky: the line is `origin + t * dir`, and each of the box's four
/// edges bounds `t` from one side. The surviving `[t_min, t_max]` window is
/// the part of the line inside the box.
fn clip_line_to_box(
    origin: Point2<CameraSpace>,
    dir: Vector2D<f64, CameraSpace>,
    page: &euclid::Box2D<f64, CameraSpace>,
) -> Option<(Point2<CameraSpace>, Point2<CameraSpace>)> {
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;

    let edges = [
        (-dir.x, origin.x - page.min.x),
        (dir.x, page.max.x - origin.x),
        (-dir.y, origin.y - page.min.y),
        (dir.y, page.max.y - origin.y),
    ];

    for (p, q) in edges {
        if p == 0.0 {
            // The line runs parallel to this edge: it either lies wholly
            // inside the slab or wholly outside it.
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t_min = t_min.max(t);
            } else {
                t_max = t_max.min(t);
            }
        }
    }

    (t_min <= t_max).then(|| (origin + dir * t_max, origin + dir * t_min))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::AxisAlignedCuboid;

    fn unit_page() -> euclid::Box2D<f64, CameraSpace> {
        euclid::Box2D::new(Point2::new(0.0, 0.0), Point2::new(10.0, 10.0))
    }

    fn cube_scene(center: f64) -> Scene {
        let cuboid = AxisAlignedCuboid::new()
            .min((center - 0.5, -0.5, -0.5))
            .max((center + 0.5, 0.5, 0.5))
            .build();
        Scene::new()
            .geometry(vec![Arc::new(cuboid) as Arc<dyn Shape>])
            .build()
    }

    #[test]
    fn an_occluder_between_the_point_and_the_eye_hides_it() {
        let scene = cube_scene(5.0);
        let point = WPoint3::new(0.0, 0.0, 0.0);
        let eye = WPoint3::new(10.0, 0.0, 0.0);

        assert!(!scene.visible(eye, point));
    }

    #[test]
    fn geometry_behind_the_eye_does_not_occlude() {
        let scene = cube_scene(20.0);
        let point = WPoint3::new(0.0, 0.0, 0.0);
        let eye = WPoint3::new(10.0, 0.0, 0.0);

        assert!(scene.visible(eye, point));
    }

    #[test]
    fn clips_a_horizontal_line_to_the_page_edges() {
        let (forward, backward) =
            clip_line_to_box(Point2::new(3.0, 4.0), Vector2D::new(1.0, 0.0), &unit_page())
                .expect("a line through the page meets it");

        assert_eq!(forward, Point2::new(10.0, 4.0));
        assert_eq!(backward, Point2::new(0.0, 4.0));
    }

    #[test]
    fn clips_a_diagonal_line_to_the_page_corners() {
        let diag = Vec2::new(1.0, 1.0).normalize();
        let (forward, backward) =
            clip_line_to_box(Point2::new(5.0, 5.0), diag, &unit_page()).expect("the diagonal fits");

        assert_eq!(forward, Point2::new(10.0, 10.0));
        assert_eq!(backward, Point2::new(0.0, 0.0));
    }

    #[test]
    fn reports_no_crossing_for_a_line_beside_the_page() {
        let outside = clip_line_to_box(
            Point2::new(-1.0, 4.0),
            Vector2D::new(0.0, 1.0),
            &unit_page(),
        );

        assert!(outside.is_none());
    }
}
