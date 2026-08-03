mod view;

use bon::Builder;
use euclid::{Transform3D, Vector3D};
use path::SlicedSegment3D;
use snafu::prelude::*;

pub use self::view::*;
use crate::*;

pub const DEFAULT_PEN_PX_SIZE: f64 = 4.0;
pub const DEFAULT_HATCH_PIXEL_SPACING_FACTOR: f64 = 2.0;
pub const DEFAULT_HATCH_PIXEL_CHOP_QUOTIENT: f64 = 3.0;
pub const DEFAULT_HATCH_SLICE_FORGIVENESS: usize = 1;
pub const DEFAULT_VERT_HATCH_BRIGHTNESS_SCALING: f64 = 0.8;
pub const DEFAULT_DIAG_HATCH_BRIGHTNESS_SCALING: f64 = 0.46;

/// Why a camera placement describes no view.
#[derive(Debug, Snafu)]
pub enum LookAtError {
    #[snafu(display("the eye and the point it looks at coincide, so there is no view direction"))]
    ZeroView,
    #[snafu(display(
        "the up direction is parallel to the view direction, so there is no camera basis"
    ))]
    UpParallelToView,
}

/// Why a set of frustum parameters describes no projection.
#[derive(Debug, Snafu)]
pub enum PerspectiveError {
    #[snafu(display(
        "the vertical field of view must be between 0 and 180 degrees, but was {fovy}"
    ))]
    FovyOutOfRange { fovy: f64 },
    #[snafu(display(
        "the render must be at least one pixel in each dimension, but was {width}x{height}"
    ))]
    EmptyViewport { width: usize, height: usize },
    #[snafu(display(
        "the clip planes must satisfy 0 < znear < zfar, but were znear {znear} and zfar {zfar}"
    ))]
    ClipPlanesOutOfOrder { znear: f64, zfar: f64 },
}

#[derive(Debug, Clone, Builder, Default)]
#[builder(start_fn(name = new))]
pub struct Camera {
    pub observation: Observation,
    pub perspective: Perspective,
    #[builder(default)]
    pub render_options: CameraOptions,
}

#[derive(Debug, Clone, Builder)]
#[builder(start_fn(name = new))]
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
        Self::new().build()
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
        let (xmax, ymax) = p.half_extents();

        let frustum = frustum(-xmax, xmax, -ymax, ymax, p.znear(), p.zfar());
        self.observation.world_to_camera_transform().then(&frustum)
    }

    #[must_use]
    pub fn camera_transformation(&self) -> Transform3D<f64, WorldSpace, CameraSpace> {
        self.canvas_transformation()
            .then_translate(Vec3::new(1.0, 1.0, 0.0))
            .then_scale(
                self.perspective.width() as f64 / 2.0,
                self.perspective.height() as f64 / 2.0,
                1.0,
            )
            .with_destination()
    }

    /// Chops a line segment into subsegments based on distance from camera
    pub fn chop_segment<'a>(
        &self,
        segment: &'a LineSegment3D<WorldSpace>,
    ) -> Option<SlicedSegment3D<'a, WorldSpace>> {
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

    /// The ray leaving the eye through the given camera-space pixel.
    ///
    /// The direction is read straight off the camera basis and the frustum
    /// extents, so no matrix has to be inverted to answer the question.
    pub fn ray_for_px_coords(&self, x: f64, y: f64) -> Ray {
        let p = &self.perspective;
        let (xmax, ymax) = p.half_extents();

        // Pixel coordinates run from 0 to the render dimensions; the frustum
        // extents describe the same window measured from its centre.
        let ndc_x = 2.0 * x / p.width() as f64 - 1.0;
        let ndc_y = 2.0 * y / p.height() as f64 - 1.0;

        let observation = &self.observation;
        let dir = observation.right() * (ndc_x * xmax)
            + observation.up() * (ndc_y * ymax)
            + observation.look() * p.znear();

        Ray::new(observation.eye(), dir.normalize())
    }

    #[must_use]
    fn min_step_size(&self) -> f64 {
        let p = &self.perspective;
        let (xmax, ymax) = p.half_extents();

        // TODO: We can apply scaling here based on pen size
        let effective_dims: Vec2<()> = Vec2::new(p.width() as f64, p.height() as f64);

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
    ) -> Result<Observation, LookAtError> {
        Observation::look_at(eye, center, up)
    }

    pub fn perspective(
        fovy: f64,
        width: usize,
        height: usize,
        znear: f64,
        zfar: f64,
    ) -> Result<Perspective, PerspectiveError> {
        Perspective::try_new(fovy, width, height, znear, zfar)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera {
        Camera::new()
            .observation(
                Camera::look_at(
                    (8.0, 6.0, 4.0),
                    WVec3::new(0.0, 0.0, 0.0),
                    WVec3::new(0.0, 0.0, 1.0),
                )
                .expect("this camera looks at a point in front of it"),
            )
            .perspective(
                Camera::perspective(50.0, 1024, 768, 0.1, 20.0)
                    .expect("these frustum parameters are well formed"),
            )
            .build()
    }

    #[test]
    fn the_ray_through_the_middle_pixel_looks_where_the_camera_looks() {
        let camera = camera();
        let ray = camera.ray_for_px_coords(512.0, 384.0);

        assert!((ray.point - camera.observation.eye()).length() < 1.0e-12);
        assert!(
            (ray.dir - camera.observation.look()).length() < 1.0e-12,
            "the middle pixel looked along {:?} instead of {:?}",
            ray.dir,
            camera.observation.look()
        );
    }

    #[test]
    fn the_ray_through_a_pixel_projects_back_onto_that_pixel() {
        let camera = camera();
        let (x, y) = (301.0, 122.0);
        let ray = camera.ray_for_px_coords(x, y);

        let along = ray.point + ray.dir * 7.0;
        let projected = camera
            .camera_transformation()
            .transform_point3d(along)
            .expect("a point in front of the camera projects");

        assert!(
            (projected.x - x).abs() < 1.0e-9 && (projected.y - y).abs() < 1.0e-9,
            "the ray for pixel ({x}, {y}) projected back to {projected:?}"
        );
    }
}
