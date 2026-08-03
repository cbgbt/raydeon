//! Where the camera sits and what it can see.
//!
//! Both types keep their fields private: an observation must stay invertible
//! and a frustum must stay well-formed, so the only way to mint either is to
//! parse one.

use super::*;

/// Below this length a basis vector is treated as degenerate: the camera
/// it would describe has no usable orientation.
const MIN_BASIS_LENGTH: f64 = 1.0e-9;

/// Where the camera sits and which way it faces.
///
/// Both the camera-to-world matrix and its inverse are kept, and every
/// mutation updates the pair, so projecting world geometry into camera
/// space never has to invert a matrix and never has to handle the failure
/// of doing so.
#[derive(Debug, Copy, Clone)]
pub struct Observation {
    view_mat: CWTransform,
    inv_view_mat: WCTransform,
}

impl Default for Observation {
    fn default() -> Self {
        Self::look_at((0.0, 0.0, 1.0), (0.0, 0.0, 0.0), (0.0, 1.0, 0.0))
            .expect("the default observation looks down -z with a perpendicular up")
    }
}

impl Observation {
    pub fn look_at(
        eye: impl Into<WPoint3>,
        center: impl Into<WVec3>,
        up: impl Into<WVec3>,
    ) -> Result<Self, LookAtError> {
        let view_mat = Self::create_view_matrix(eye, center, up)?;
        Ok(Self {
            inv_view_mat: rigid_inverse(&view_mat),
            view_mat,
        })
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
        self.set_view_matrix(
            self.view_mat
                .pre_rotate(axis.x, axis.y, axis.z, angle)
                .with_destination(),
        );
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
        self.set_view_matrix(
            self.view_mat
                .pre_translate(trans.cast_unit())
                .with_destination(),
        );
    }

    /// Replaces the view matrix, keeping the stored inverse in step.
    fn set_view_matrix(&mut self, view_mat: CWTransform) {
        self.inv_view_mat = rigid_inverse(&view_mat);
        self.view_mat = view_mat;
    }

    #[rustfmt::skip]
    fn create_view_matrix(
        eye: impl Into<WPoint3>,
        center: impl Into<WVec3>,
        up: impl Into<WVec3>,
    ) -> Result<CWTransform, LookAtError> {
        let eye = eye.into();
        let center = center.into();
        let up = up.into().normalize();

        let view = center - eye.to_vector();
        ensure!(view.length() > MIN_BASIS_LENGTH, ZeroViewSnafu);
        let f = view.normalize();

        let s = f.cross(up);
        ensure!(s.length() > MIN_BASIS_LENGTH, UpParallelToViewSnafu);
        let s = s.normalize();
        let u = s.cross(f).normalize();

        Ok(CWTransform::new(
            s.x, s.y, s.z, 0.0,
            u.x, u.y, u.z, 0.0,
            -f.x, -f.y, -f.z, 0.0,
            eye.x, eye.y, eye.z, 1.0
        ))
    }

    pub(crate) fn world_to_camera_transform(&self) -> WCTransform {
        self.inv_view_mat
    }
}

/// Inverts a rigid transform (an orthonormal rotation plus a translation)
/// by transposing its rotation and undoing its translation, which is exact
/// and cannot fail — unlike a general matrix inverse.
#[rustfmt::skip]
fn rigid_inverse(m: &CWTransform) -> WCTransform {
    let t = (
        -(m.m41 * m.m11 + m.m42 * m.m12 + m.m43 * m.m13),
        -(m.m41 * m.m21 + m.m42 * m.m22 + m.m43 * m.m23),
        -(m.m41 * m.m31 + m.m42 * m.m32 + m.m43 * m.m33),
    );

    WCTransform::new(
        m.m11, m.m21, m.m31, 0.0,
        m.m12, m.m22, m.m32, 0.0,
        m.m13, m.m23, m.m33, 0.0,
        t.0,   t.1,   t.2,   1.0,
    )
}

/// The viewing frustum: field of view, render dimensions, and clip planes.
///
/// Fields are private because the frustum they describe must be
/// invertible; [`Perspective::try_new`] is the only way to mint one.
#[derive(Debug, Copy, Clone)]
pub struct Perspective {
    fovy: f64,
    width: usize,
    height: usize,
    aspect: f64,
    znear: f64,
    zfar: f64,
}

impl Default for Perspective {
    fn default() -> Self {
        Self::try_new(45.0, 1920, 1080, 0.1, 100.0)
            .expect("the default perspective describes a valid frustum")
    }
}

impl Perspective {
    pub fn try_new(
        fovy: f64,
        width: usize,
        height: usize,
        znear: f64,
        zfar: f64,
    ) -> Result<Self, PerspectiveError> {
        ensure!(fovy > 0.0 && fovy < 180.0, FovyOutOfRangeSnafu { fovy });
        ensure!(
            width >= 1 && height >= 1,
            EmptyViewportSnafu { width, height }
        );
        ensure!(
            znear > 0.0 && znear < zfar,
            ClipPlanesOutOfOrderSnafu { znear, zfar }
        );

        let aspect = width as f64 / height as f64;
        Ok(Self {
            fovy,
            width,
            height,
            aspect,
            znear,
            zfar,
        })
    }

    pub fn fovy(&self) -> f64 {
        self.fovy
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn aspect(&self) -> f64 {
        self.aspect
    }

    pub fn znear(&self) -> f64 {
        self.znear
    }

    pub fn zfar(&self) -> f64 {
        self.zfar
    }

    /// Half-width and half-height of the frustum window at the near plane.
    pub(crate) fn half_extents(&self) -> (f64, f64) {
        let ymax = self.znear * (self.fovy * std::f64::consts::PI / 360.0).tan();
        (ymax * self.aspect, ymax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_eye_at_the_point_it_looks_at_has_no_view() {
        let observation = Observation::look_at(
            (1.0, 2.0, 3.0),
            WVec3::new(1.0, 2.0, 3.0),
            WVec3::new(0.0, 0.0, 1.0),
        );
        assert!(matches!(observation, Err(LookAtError::ZeroView)));
    }

    #[test]
    fn an_up_along_the_view_direction_has_no_basis() {
        let observation = Observation::look_at(
            (0.0, 0.0, 5.0),
            WVec3::new(0.0, 0.0, 0.0),
            WVec3::new(0.0, 0.0, 1.0),
        );
        assert!(matches!(observation, Err(LookAtError::UpParallelToView)));
    }

    #[test]
    fn the_stored_inverse_undoes_the_view_matrix_after_every_mutation() {
        let mut observation = Observation::look_at(
            (8.0, 6.0, 4.0),
            WVec3::new(0.0, 0.0, 0.0),
            WVec3::new(0.0, 0.0, 1.0),
        )
        .expect("this camera looks at a point in front of it");
        observation.adjust_yaw(euclid::Angle::degrees(-37.0));
        observation.adjust_pitch(euclid::Angle::degrees(11.0));
        observation.translate(WVec3::new(-3.0, 2.5, 1.0));

        let round_trip = observation
            .view_mat
            .then(&observation.world_to_camera_transform())
            .with_destination::<CameraSpace>();
        let point = CPoint3::new(1.5, -2.5, 4.0);
        let mapped = round_trip
            .transform_point3d(point)
            .expect("a rigid transform maps every point");

        assert!(
            (mapped - point).length() < 1.0e-12,
            "the inverse did not undo the view matrix: {mapped:?} vs {point:?}"
        );
    }

    #[test]
    fn a_field_of_view_outside_zero_to_half_a_turn_is_no_projection() {
        assert!(matches!(
            Perspective::try_new(0.0, 64, 64, 0.1, 10.0),
            Err(PerspectiveError::FovyOutOfRange { .. })
        ));
        assert!(matches!(
            Perspective::try_new(180.0, 64, 64, 0.1, 10.0),
            Err(PerspectiveError::FovyOutOfRange { .. })
        ));
    }

    #[test]
    fn a_viewport_without_pixels_is_no_projection() {
        assert!(matches!(
            Perspective::try_new(50.0, 0, 64, 0.1, 10.0),
            Err(PerspectiveError::EmptyViewport { .. })
        ));
        assert!(matches!(
            Perspective::try_new(50.0, 64, 0, 0.1, 10.0),
            Err(PerspectiveError::EmptyViewport { .. })
        ));
    }

    #[test]
    fn clip_planes_must_be_positive_and_ordered() {
        assert!(matches!(
            Perspective::try_new(50.0, 64, 64, 0.0, 10.0),
            Err(PerspectiveError::ClipPlanesOutOfOrder { .. })
        ));
        assert!(matches!(
            Perspective::try_new(50.0, 64, 64, 10.0, 0.1),
            Err(PerspectiveError::ClipPlanesOutOfOrder { .. })
        ));
    }
}
