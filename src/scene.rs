use bvh::BVHTree;
use path::{simplify_segments, transform_segment};
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::*;

pub struct LookingCamera {
    eye: WPoint3,
    center: WVec3,
    up: WVec3,
    matrix: WCTransform,
}

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    eye: WPoint3,
    _center: WVec3,
    _up: WVec3,

    _fovy: f64,
    width: f64,
    height: f64,
    _aspect: f64,
    znear: f64,
    zfar: f64,

    min_step_size: f64,
    max_step_size: f64,

    matrix: Transform3D<f64, WorldSpace, CanvasSpace>,
}

impl Camera {
    // TODO testme
    #[rustfmt::skip]
    pub fn look_at(eye: WPoint3, center: WVec3, up: WVec3) -> LookingCamera {
        let up = up.normalize();
        let f = (center - eye.to_vector()).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f).normalize();

        let look_at_matrix = CWTransform::from_array(
            // euclid used to let us specify things in column major order and now it doesn't.
            // So we're just transposing it here.
            CWTransform::new(
                s.x, u.x, -f.x, eye.x,
                s.y, u.y, -f.y, eye.y,
                s.z, u.z, -f.z, eye.z,
                0.0, 0.0, 0.0,  1.0,
            )
            .to_array_transposed(),
        )
        .inverse()
        .unwrap();

        let matrix = look_at_matrix;
        LookingCamera {
            eye,
            center,
            up,
            matrix,
        }
    }

    /// Chops a line segment into subsegments based on distance from camera
    pub fn chop_segment(&self, segment: &LineSegment<WorldSpace>) -> Vec<LineSegment<WorldSpace>> {
        // linearly interpolate step_size based on closest point to the camera
        let (p1, p2) = segment;
        let p1 = p1.to_vector();
        let p2 = p2.to_vector();
        let segment_diff = p2 - p1;
        let midpoint = p1 + (segment_diff / 2.0);

        let eye = self.eye.to_vector();

        let t1 = (p1 - eye).length();
        let t2 = (p2 - eye).length();
        let t3 = (midpoint - eye).length();

        let closest = f64::min(f64::min(t1, t2), t3);

        let plane_dist = self.zfar - self.znear;
        let scale = (closest - self.znear) / plane_dist;

        let rough_step_size =
            self.min_step_size + (scale * (self.max_step_size - self.min_step_size));

        // Slice segment into equal-sized chunks of approx `rough_step_size` length
        let segment_length = segment_diff.length();
        if segment_length < rough_step_size {
            return Vec::new();
        }

        let chunk_count = usize::max(
            (segment_length / rough_step_size).round_ties_even() as usize,
            1,
        );
        if chunk_count == 1 {
            return vec![(p1.to_point(), p2.to_point())];
        }

        let true_chunk_len = segment_length / chunk_count as f64;
        let true_chunk_len = f64::min(true_chunk_len, segment_length);

        let segment_dir = segment_diff.normalize();
        let chunk_vec = segment_dir * true_chunk_len;
        (0..chunk_count)
            .map(|segment_ndx| {
                let p1 = segment.0 + (chunk_vec * (segment_ndx as f64));
                let p2 = p1 + chunk_vec;
                (p1, p2)
            })
            .collect()
    }
}

#[rustfmt::skip]
fn frustum(
    l: f64,
    r: f64,
    b: f64,
    t: f64,
    n: f64,
    f: f64,
) -> Transform3D<f64, CameraSpace, CanvasSpace> {
    let t1 = 2.0 * n;
    let t2 = r - l;
    let t3 = t - b;
    let t4 = f - n;

    // euclid used to let us specify things in column major order and now it doesn't.
    // So we're just transposing it here.
    Transform3D::from_array(
        Transform3D::<f64, CameraSpace, CanvasSpace>::new(
            t1 / t2, 0.0,     (r + l) / t2, 0.0,
            0.0,     t1 / t3, (t + b) / t3, 0.0,
            0.0,     0.0,     (-f - n) / t4,(-t1 * f) / t4,
            0.0,     0.0,     -1.0,         0.0,
        )
        .to_array_transposed(),
    )
}

impl LookingCamera {
    pub fn perspective(self, fovy: f64, width: f64, height: f64, znear: f64, zfar: f64) -> Camera {
        let aspect = width / height;
        let ymax = znear * (fovy * std::f64::consts::PI / 360.0).tan();
        let xmax = ymax * aspect;

        let frustum = frustum(-xmax, xmax, -ymax, ymax, znear, zfar);
        let matrix = self.matrix.then(&frustum);

        let effective_width = width;
        let effective_height = height;
        let znear_width = 2.0 * xmax;
        let znear_height = 2.0 * ymax;
        let est_min_pix_height = (znear_height / effective_height);
        let est_min_pix_width = (znear_width / effective_width);

        let min_step_size = f64::min(est_min_pix_height, est_min_pix_width);

        let zfar_ymax = zfar * (fovy * std::f64::consts::PI / 360.0).tan();
        let zfar_xmax = zfar_ymax * aspect;

        let zfar_width = 2.0 * zfar_xmax;
        let zfar_height = 2.0 * zfar_ymax;
        let est_max_pix_height = zfar_height / effective_height;
        let est_max_pix_width = zfar_width / effective_width;

        let max_step_size = f64::min(est_max_pix_height, est_max_pix_width);

        Camera {
            eye: self.eye,
            _center: self.center,
            _up: self.up,
            _fovy: fovy,
            width,
            height,
            _aspect: aspect,
            znear,
            zfar,
            min_step_size,
            max_step_size,
            matrix,
        }
    }
}

pub struct SceneCamera<'s> {
    camera: Camera,
    scene: &'s Scene,
    paths: Vec<Vec<LineSegment<WorldSpace>>>,
    path_count: usize,
}

impl<'a> SceneCamera<'a> {
    fn clip_filter(&self, path: &LineSegment<WorldSpace>) -> bool {
        let (p1, p2) = path;
        let midpoint = *p1 + ((*p2 - *p1) / 2.0);
        self.scene.visible(self.camera.eye, midpoint)
    }

    pub fn render(&self) -> Vec<LineSegment<CameraSpace>> {
        info!(
            "Clipping occluded and distant segment chunks, started with {} segments",
            self.path_count
        );

        let transformation: Transform3D<f64, WorldSpace, CameraSpace> = self
            .camera
            .matrix
            .then_translate(Vector3D::new(1.0, 1.0, 0.0))
            .then_scale(self.camera.width / 2.0, self.camera.height / 2.0, 0.0)
            .with_destination();

        let paths: Vec<_> = self
            .paths
            .par_iter()
            .map(|path_group| {
                path_group
                    .par_iter()
                    .filter(|path| {
                        let close_enough = (path.0.to_vector() - self.camera.eye.to_vector())
                            .length()
                            < self.camera.zfar;
                        close_enough && self.clip_filter(path)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flat_map(|path_group| simplify_segments(&path_group, 1.0e-6))
            .filter_map(|path| transform_segment(path, &transformation))
            .collect();

        info!("{} paths remain after clipping", paths.len());

        paths
    }
}

#[derive(Debug)]
pub struct Scene {
    geometry: Vec<Arc<dyn Shape<WorldSpace>>>,
    bvh: BVHTree<WorldSpace>,
}

impl Scene {
    pub fn new(geometry: Vec<Box<dyn Shape<WorldSpace>>>) -> Scene {
        let geometry: Vec<_> = geometry.into_iter().map(Arc::from).collect();
        let bvh = BVHTree::new(&geometry);
        Scene { geometry, bvh }
    }

    pub fn attach_camera(&self, camera: Camera) -> SceneCamera {
        info!("Caching line segment chunks based on new camera attachment");
        let paths: Vec<Vec<LineSegment<WorldSpace>>> = self
            .geometry
            .par_iter()
            .map(|s| s.paths())
            .flat_map(|paths| {
                paths
                    .par_iter()
                    .map(|path| camera.chop_segment(path))
                    .collect::<Vec<_>>()
            })
            .collect();
        let path_count = paths.par_iter().map(|path_group| path_group.len()).sum();
        info!(
            "Done caching line segment chunks, created {} segment chunks",
            path_count
        );

        SceneCamera {
            camera,
            scene: self,
            paths,
            path_count,
        }
    }

    /// Find's the closest intersection point to geometry in the scene, if any
    fn intersects(&self, ray: Ray) -> Option<HitData> {
        self.bvh.intersects(ray)
    }

    /// Returns whether or not the given camera has a clear line of sight to a given point.
    fn visible(&self, from: WPoint3, point: WPoint3) -> bool {
        let v = from - point;
        let r = Ray::new(point, v.normalize());

        match self.intersects(r) {
            Some(hitdata) => {
                let diff = (hitdata.dist_to - v.length()).abs();
                diff < 1.0e-1
            }
            None => true,
        }
    }
}
