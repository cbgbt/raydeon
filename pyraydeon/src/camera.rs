use numpy::{Ix1, PyArray};
use pyo3::prelude::*;

use crate::linear::{Point3, Vec3};

#[derive(Debug, Clone)]
#[pyclass(frozen)]
pub(crate) struct Camera(pub(crate) raydeon::Camera);

impl ::std::ops::Deref for Camera {
    type Target = raydeon::Camera;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<raydeon::Camera> for Camera {
    fn from(value: raydeon::Camera) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Camera {
    #[new]
    fn new() -> Self {
        raydeon::Camera::default().into()
    }

    fn look_at(
        &self,
        eye: &Bound<'_, PyAny>,
        center: &Bound<'_, PyAny>,
        up: &Bound<'_, PyAny>,
    ) -> PyResult<Camera> {
        let eye = Point3::try_from(eye)?;
        let center = Vec3::try_from(center)?;
        let up = Vec3::try_from(up)?;
        let mut ncam = self.0.clone();
        ncam.observation =
            raydeon::Camera::look_at(eye.cast_unit(), center.cast_unit(), up.cast_unit());
        Ok(ncam.into())
    }

    fn perspective(&self, fovy: f64, width: usize, height: usize, znear: f64, zfar: f64) -> Camera {
        let mut ncam = self.0.clone();
        ncam.perspective = raydeon::Camera::perspective(fovy, width, height, znear, zfar);
        ncam.into()
    }

    fn render_options(&self, options: CameraOptions) -> Camera {
        let mut ncam = self.0.clone();
        ncam.render_options = options.0.clone();
        ncam.into()
    }

    #[getter]
    fn eye<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.observation.eye.to_array())
    }

    #[getter]
    fn focus<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.observation.center.to_array())
    }

    #[getter]
    fn up<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.observation.up.to_array())
    }

    #[getter]
    fn fovy(&self) -> f64 {
        self.0.perspective.fovy
    }

    #[getter]
    fn width(&self) -> usize {
        self.0.perspective.width
    }

    #[getter]
    fn height(&self) -> usize {
        self.0.perspective.height
    }

    #[getter]
    fn aspect(&self) -> f64 {
        self.0.perspective.aspect
    }

    #[getter]
    fn znear(&self) -> f64 {
        self.0.perspective.znear
    }

    #[getter]
    fn zfar(&self) -> f64 {
        self.0.perspective.zfar
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

#[derive(Debug, Clone)]
#[pyclass]
pub(crate) struct CameraOptions(pub(crate) raydeon::CameraOptions);

impl ::std::ops::Deref for CameraOptions {
    type Target = raydeon::CameraOptions;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<raydeon::CameraOptions> for CameraOptions {
    fn from(value: raydeon::CameraOptions) -> Self {
        Self(value)
    }
}

#[pymethods]
impl CameraOptions {
    #[new]
    #[pyo3(signature = (
        pen_px_size=None,
        hatch_pixel_spacing=None,
        hatch_pixel_chop_factor=None,
        hatch_slice_forgiveness=None,
        vert_hatch_brightness_scaling=None,
        diag_hatch_brightness_scaling=None
    ))]
    fn new(
        pen_px_size: Option<f64>,
        hatch_pixel_spacing: Option<f64>,
        hatch_pixel_chop_factor: Option<f64>,
        hatch_slice_forgiveness: Option<usize>,
        vert_hatch_brightness_scaling: Option<f64>,
        diag_hatch_brightness_scaling: Option<f64>,
    ) -> Self {
        raydeon::CameraOptions::configure()
            .maybe_pen_px_size(pen_px_size)
            .maybe_hatch_pixel_spacing(hatch_pixel_spacing)
            .maybe_hatch_pixel_chop_factor(hatch_pixel_chop_factor)
            .maybe_hatch_slice_forgiveness(hatch_slice_forgiveness)
            .maybe_vert_hatch_brightness_scaling(vert_hatch_brightness_scaling)
            .maybe_diag_hatch_brightness_scaling(diag_hatch_brightness_scaling)
            .build()
            .into()
    }

    #[getter]
    fn get_pen_px_size(&self) -> f64 {
        self.0.pen_px_size
    }

    #[setter]
    fn set_pen_px_size(&mut self, pen_px_size: f64) {
        self.0.pen_px_size = pen_px_size;
    }

    #[getter]
    fn get_hatch_pixel_spacing(&self) -> f64 {
        self.0.hatch_pixel_spacing
    }

    #[setter]
    fn set_hatch_pixel_spacing(&mut self, hatch_pixel_spacing: f64) {
        self.0.hatch_pixel_spacing = hatch_pixel_spacing;
    }

    #[getter]
    fn get_hatch_pixel_chop_factor(&self) -> f64 {
        self.0.hatch_pixel_chop_factor
    }

    #[setter]
    fn set_hatch_pixel_chop_factor(&mut self, hatch_pixel_chop_factor: f64) {
        self.0.hatch_pixel_chop_factor = hatch_pixel_chop_factor;
    }

    #[getter]
    fn get_hatch_slice_forgiveness(&self) -> usize {
        self.0.hatch_slice_forgiveness
    }

    #[setter]
    fn set_hatch_slice_forgiveness(&mut self, hatch_slice_forgiveness: usize) {
        self.0.hatch_slice_forgiveness = hatch_slice_forgiveness;
    }

    #[getter]
    fn get_vert_hatch_brightness_scaling(&self) -> f64 {
        self.0.vert_hatch_brightness_scaling
    }

    #[setter]
    fn set_vert_hatch_brightness_scaling(&mut self, vert_hatch_brightness_scaling: f64) {
        self.0.vert_hatch_brightness_scaling = vert_hatch_brightness_scaling;
    }

    #[getter]
    fn get_diag_hatch_brightness_scaling(&self) -> f64 {
        self.0.diag_hatch_brightness_scaling
    }

    #[setter]
    fn set_diag_hatch_brightness_scaling(&mut self, diag_hatch_brightness_scaling: f64) {
        self.0.diag_hatch_brightness_scaling = diag_hatch_brightness_scaling;
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().0))
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Camera>()?;
    m.add_class::<CameraOptions>()?;
    Ok(())
}
