use euclid::{Point3D, Transform3D};
use path::SlicedSegment3D;

use crate::*;

#[derive(Debug, Copy, Clone)]
pub struct Observation {
    pub eye: WPoint3,
    pub center: WVec3,
    pub up: WVec3,
}

impl Default for Observation {
    fn default() -> Self {
        Self::new((0.0, 0.0, 1.0), (0.0, 0.0, 0.0), (0.0, 1.0, 0.0))
    }
}

/// Type parameter for a camera that isn't yet looking anywhere
#[derive(Debug, Copy, Clone)]
pub struct NoObservation;

impl Observation {
    pub fn new(eye: impl Into<WPoint3>, center: impl Into<WVec3>, up: impl Into<WVec3>) -> Self {
        let eye = eye.into();
        let center = center.into();
        let up = up.into().normalize();

        Self { eye, center, up }
    }

    #[rustfmt::skip]
    pub fn look_matrix(&self) -> WCTransform {
        let Observation { eye, center, up, .. } = *self;
        let f = (center - eye.to_vector()).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f).normalize();

        CWTransform::from_array(
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
        .unwrap()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Perspective {
    pub fovy: f64,
    pub width: usize,
    pub height: usize,
    pub aspect: f64,
    pub znear: f64,
    pub zfar: f64,
}

impl Default for Perspective {
    fn default() -> Self {
        Self::new(45.0, 1920, 1080, 0.1, 100.0)
    }
}

/// Type parameter for a camera that doesn't yet have a defined perspective
#[derive(Debug, Copy, Clone)]
pub struct NoPerspective;

impl Perspective {
    pub fn new(fovy: f64, width: usize, height: usize, znear: f64, zfar: f64) -> Self {
        let aspect = width as f64 / height as f64;
        Self {
            fovy,
            width,
            height,
            aspect,
            znear,
            zfar,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Camera<P, O> {
    pub observation: O,
    pub perspective: P,
}

impl Camera<NoPerspective, NoObservation> {
    pub fn new() -> Self {
        Self {
            observation: NoObservation,
            perspective: NoPerspective,
        }
    }
}

impl Default for Camera<Perspective, Observation> {
    fn default() -> Self {
        Self {
            observation: Observation::default(),
            perspective: Perspective::default(),
        }
    }
}

impl<P, O> Camera<P, O> {
    pub fn look_at(
        self,
        eye: impl Into<WPoint3>,
        center: impl Into<WVec3>,
        up: impl Into<WVec3>,
    ) -> Camera<P, Observation> {
        let Camera { perspective, .. } = self;
        let observation = Observation::new(eye.into(), center.into(), up.into());
        Camera {
            observation,
            perspective,
        }
    }

    pub fn perspective(
        self,
        fovy: f64,
        width: usize,
        height: usize,
        znear: f64,
        zfar: f64,
    ) -> Camera<Perspective, O> {
        let Camera { observation, .. } = self;
        let perspective = Perspective::new(fovy, width, height, znear, zfar);
        Camera {
            observation,
            perspective,
        }
    }
}

impl Camera<Perspective, Observation> {
    #[must_use]
    pub fn canvas_transformation(&self) -> Transform3D<f64, WorldSpace, CanvasSpace> {
        let p = &self.perspective;
        let ymax = p.znear * (p.fovy * std::f64::consts::PI / 360.0).tan();
        let xmax = ymax * p.aspect;

        let frustum = frustum(-xmax, xmax, -ymax, ymax, p.znear, p.zfar);
        self.observation.look_matrix().then(&frustum)
    }

    #[must_use]
    pub fn camera_transformation(&self) -> Transform3D<f64, WorldSpace, CameraSpace> {
        self.canvas_transformation()
            .then_translate(Vec3::new(1.0, 1.0, 0.0))
            .then_scale(
                self.perspective.width as f64 / 2.0,
                self.perspective.height as f64 / 2.0,
                1.0,
            )
            .with_destination()
    }

    /// Chops a line segment into subsegments based on distance from camera
    pub fn chop_segment<'a, P: PathMeta>(
        &self,
        segment: &'a LineSegment3D<WorldSpace, P>,
    ) -> Option<SlicedSegment3D<'a, WorldSpace, P>> {
        let p1 = segment.p1().to_vector();
        let p2 = segment.p2().to_vector();

        // Transform the points to camera space, then chop based on the pixel length
        let transformation = self.camera_transformation();
        let canvas_points = transformation
            .transform_point3d(p1.to_point())
            .and_then(|p1t| {
                transformation
                    .transform_point3d(p2.to_point())
                    .map(|p2t| (p1t.xy(), p2t.xy()))
            });

        let chunk_count = canvas_points
            .map(|(p1t, p2t)| {
                let rough_chop_size = (p2t - p1t).length() / (PEN_PX_SIZE / 2.0);
                rough_chop_size.round_ties_even() as usize
            })
            .unwrap_or_else(|| {
                let rough_chop_size = self.min_step_size();
                ((p2 - p1).length() / rough_chop_size).round_ties_even() as usize
            });

        if chunk_count == 0 {
            None
        } else {
            Some(SlicedSegment3D::new(chunk_count, segment))
        }
    }

    pub fn ray_for_px_coords(&self, x: f64, y: f64) -> Ray {
        let pix_ndc = Point3D::new(x, y, 0.0);

        let world_coord = self
            .camera_transformation()
            .inverse()
            .unwrap()
            .transform_point3d(pix_ndc)
            .unwrap();

        Ray {
            point: self.observation.eye,
            dir: (world_coord.to_vector() - self.observation.eye.to_vector()).normalize(),
        }
    }
}

impl<O> Camera<Perspective, O> {
    #[must_use]
    fn min_step_size(&self) -> f64 {
        let p = &self.perspective;
        let ymax = p.znear * (p.fovy * std::f64::consts::PI / 360.0).tan();
        let xmax = ymax * p.aspect;

        // TODO: We can apply scaling here based on pen size
        let effective_dims: Vec2<()> = Vec2::new(p.width as f64, p.height as f64);

        let znear_dims = Vec2::new(xmax, ymax) * 2.0;
        let est_min_pix = znear_dims.component_div(effective_dims);
        f64::min(est_min_pix.x, est_min_pix.y)
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
