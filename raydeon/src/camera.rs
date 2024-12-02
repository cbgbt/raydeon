use bon::Builder;
use euclid::{Point3D, Transform3D, Vector3D};
use path::SlicedSegment3D;

use self::view_matrix_settings::*;
use crate::*;

pub const DEFAULT_PEN_PX_SIZE: f64 = 4.0;
pub const DEFAULT_HATCH_PIXEL_SPACING_FACTOR: f64 = 2.0;
pub const DEFAULT_HATCH_PIXEL_CHOP_QUOTIENT: f64 = 3.0;
pub const DEFAULT_HATCH_SLICE_FORGIVENESS: usize = 1;
pub const DEFAULT_VERT_HATCH_BRIGHTNESS_SCALING: f64 = 0.8;
pub const DEFAULT_DIAG_HATCH_BRIGHTNESS_SCALING: f64 = 0.46;

#[derive(Debug, Clone, Builder, Default)]
#[builder(start_fn(name = configure))]
pub struct Camera {
    pub observation: Observation,
    pub perspective: Perspective,
    #[builder(default)]
    pub render_options: CameraOptions,
}

#[derive(Debug, Clone, Builder)]
#[builder(start_fn(name = configure))]
pub struct CameraOptions {
    #[builder(default = DEFAULT_PEN_PX_SIZE)]
    pub pen_px_size: f64,
    #[builder(default = pen_px_size * DEFAULT_HATCH_PIXEL_SPACING_FACTOR)]
    pub hatch_pixel_spacing: f64,
    #[builder(default = pen_px_size / DEFAULT_HATCH_PIXEL_CHOP_QUOTIENT)]
    pub hatch_pixel_chop_factor: f64,
    #[builder(default = DEFAULT_HATCH_SLICE_FORGIVENESS)]
    pub hatch_slice_forgiveness: usize,
    #[builder(default = DEFAULT_VERT_HATCH_BRIGHTNESS_SCALING)]
    pub vert_hatch_brightness_scaling: f64,
    #[builder(default = DEFAULT_DIAG_HATCH_BRIGHTNESS_SCALING)]
    pub diag_hatch_brightness_scaling: f64,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self::configure().build()
    }
}

impl Camera {
    pub fn adjust_yaw(&mut self, yaw: euclid::Angle<f64>) {
        self.observation.adjust_yaw(yaw);
    }

    pub fn adjust_pitch(&mut self, pitch: euclid::Angle<f64>) {
        self.observation.adjust_pitch(pitch);
    }

    pub fn adjust_roll(&mut self, roll: euclid::Angle<f64>) {
        self.observation.adjust_roll(roll);
    }

    pub fn translate(&mut self, trans: impl Into<Vector3D<f64, ()>>) {
        self.observation.translate(trans);
    }

    #[must_use]
    pub fn canvas_transformation(&self) -> Transform3D<f64, WorldSpace, CanvasSpace> {
        let p = &self.perspective;
        let ymax = p.znear * (p.fovy * std::f64::consts::PI / 360.0).tan();
        let xmax = ymax * p.aspect;

        let frustum = frustum(-xmax, xmax, -ymax, ymax, p.znear, p.zfar);
        self.observation.world_to_camera_transform().then(&frustum)
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
    pub fn chop_segment<'a, 's>(
        &self,
        segment: &'a LineSegment3D<'s, WorldSpace>,
    ) -> Option<SlicedSegment3D<'a, 's, WorldSpace>> {
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
                let rough_chop_size =
                    (p2t - p1t).length() / (self.render_options.pen_px_size / 2.0);
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
            point: self.observation.eye(),
            dir: (world_coord.to_vector() - self.observation.eye().to_vector()).normalize(),
        }
    }

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

// Expose constructors for view matrix args through `Camera`
impl Camera {
    pub fn look_at(
        eye: impl Into<WPoint3>,
        center: impl Into<WVec3>,
        up: impl Into<WVec3>,
    ) -> Observation {
        Observation::look_at(eye, center, up)
    }

    pub fn perspective(
        fovy: f64,
        width: usize,
        height: usize,
        znear: f64,
        zfar: f64,
    ) -> Perspective {
        let aspect = width as f64 / height as f64;
        Perspective {
            fovy,
            width,
            height,
            aspect,
            znear,
            zfar,
        }
    }
}

mod view_matrix_settings {
    use super::*;

    #[derive(Debug, Copy, Clone)]
    pub struct Observation {
        view_mat: CWTransform,
    }

    impl Default for Observation {
        fn default() -> Self {
            Self::look_at((0.0, 0.0, 1.0), (0.0, 0.0, 0.0), (0.0, 1.0, 0.0))
        }
    }

    impl Observation {
        pub fn look_at(
            eye: impl Into<WPoint3>,
            center: impl Into<WVec3>,
            up: impl Into<WVec3>,
        ) -> Self {
            let view_mat = Self::create_view_matrix(eye, center, up);
            Self { view_mat }
        }

        pub fn eye(&self) -> WPoint3 {
            (self.view_mat.m41, self.view_mat.m42, self.view_mat.m43).into()
        }

        pub fn right(&self) -> WVec3 {
            (self.view_mat.m11, self.view_mat.m12, self.view_mat.m13).into()
        }

        pub fn up(&self) -> WVec3 {
            (self.view_mat.m21, self.view_mat.m22, self.view_mat.m23).into()
        }

        pub fn look(&self) -> WVec3 {
            (-self.view_mat.m31, -self.view_mat.m32, -self.view_mat.m33).into()
        }

        fn rotate_around_axis(&mut self, axis: impl Into<WVec3>, angle: euclid::Angle<f64>) {
            let axis = axis.into();
            self.view_mat = self
                .view_mat
                .pre_rotate(axis.x, axis.y, axis.z, angle)
                .with_destination();
        }

        pub fn adjust_yaw(&mut self, yaw: euclid::Angle<f64>) {
            self.rotate_around_axis((0.0, 1.0, 0.0), yaw);
        }

        pub fn adjust_pitch(&mut self, pitch: euclid::Angle<f64>) {
            self.rotate_around_axis((1.0, 0.0, 0.0), pitch);
        }

        pub fn adjust_roll(&mut self, roll: euclid::Angle<f64>) {
            self.rotate_around_axis((0.0, 0.0, 1.0), roll);
        }

        pub fn translate<T>(&mut self, trans: impl Into<Vector3D<f64, T>>) {
            // input is a camera translation, but we're describing a
            // world translation, so we negate
            let trans = trans.into();
            self.view_mat = self
                .view_mat
                .pre_translate(trans.cast_unit())
                .with_destination();
        }

        #[rustfmt::skip]
        fn create_view_matrix(
            eye: impl Into<WPoint3>,
            center: impl Into<WVec3>,
            up: impl Into<WVec3>,
        ) -> Transform3D<f64, CameraSpace, WorldSpace> {
            let eye = eye.into();
            let center = center.into();
            let up = up.into().normalize();

            let f = (center - eye.to_vector()).normalize();
            let s = f.cross(up).normalize();
            let u = s.cross(f).normalize();

            CWTransform::new(
                s.x, s.y, s.z, 0.0,
                u.x, u.y, u.z, 0.0,
                -f.x, -f.y, -f.z, 0.0,
                eye.x, eye.y, eye.z, 1.0
            )
        }

        pub(super) fn world_to_camera_transform(&self) -> WCTransform {
            self.view_mat.inverse().unwrap()
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
