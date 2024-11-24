use std::sync::Arc;

use numpy::{Ix1, PyArray, PyReadonlyArray1};
use pyo3::prelude::*;
use raydeon::WorldSpace;

use crate::linear::{ArbitrarySpace, Point2, Point3, Vec3};
use crate::shapes::Geometry;
use crate::Material;

pywrap!(Camera, raydeon::Camera<raydeon::Perspective, raydeon::Observation>);

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
        Ok(self
            .0
            .look_at(eye.cast_unit(), center.cast_unit(), up.cast_unit())
            .into())
    }

    fn perspective(&self, fovy: f64, width: f64, height: f64, znear: f64, zfar: f64) -> Camera {
        self.0.perspective(fovy, width, height, znear, zfar).into()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

#[pyclass(frozen)]
pub(crate) struct Scene {
    scene: Arc<raydeon::Scene<Material>>,
}

#[pymethods]
impl Scene {
    #[new]
    fn new(py: Python, geometry: Vec<PyObject>) -> PyResult<Self> {
        let geometry: Vec<Arc<dyn raydeon::Shape<WorldSpace, Material>>> = geometry
            .into_iter()
            .map(|g| {
                let geom: Py<Geometry> = g.extract(py)?;
                let raydeon_shape = geom.borrow(py);
                let raydeon_shape = raydeon_shape.geometry(g);
                Ok(raydeon_shape)
            })
            .collect::<PyResult<_>>()?;
        let scene = Arc::new(raydeon::Scene::new(geometry));
        Ok(Self { scene })
    }

    fn render(&self, py: Python, camera: &Camera) -> Vec<LineSegment2D> {
        py.allow_threads(|| {
            let cam = self.scene.attach_camera(camera.0);
            cam.render()
                .into_iter()
                .map(|ls| ls.cast_unit().into())
                .collect()
        })
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().scene))
    }
}

pywrap!(LineSegment2D, raydeon::path::LineSegment2D<ArbitrarySpace>);

#[pymethods]
impl LineSegment2D {
    #[new]
    fn new(p1: PyReadonlyArray1<f64>, p2: PyReadonlyArray1<f64>) -> PyResult<Self> {
        let p1 = Point2::try_from(p1)?;
        let p2 = Point2::try_from(p2)?;
        Ok(raydeon::path::LineSegment2D::new(p1.cast_unit(), p2.cast_unit()).into())
    }

    #[getter]
    fn p1<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.p1.to_array())
    }

    #[getter]
    fn p2<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.p2.to_array())
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pywrap!(LineSegment3D, raydeon::path::LineSegment3D<ArbitrarySpace, Material>);

#[pymethods]
impl LineSegment3D {
    #[new]
    fn new(p1: &Bound<'_, PyAny>, p2: &Bound<'_, PyAny>) -> PyResult<Self> {
        let p1 = Point3::try_from(p1)?;
        let p2 = Point3::try_from(p2)?;
        Ok(raydeon::path::LineSegment3D::tagged(p1.cast_unit(), p2.cast_unit(), Material).into())
    }

    #[getter]
    fn p1<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.p1().to_array())
    }

    #[getter]
    fn p2<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.p2().to_array())
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Camera>()?;
    m.add_class::<Scene>()?;
    m.add_class::<LineSegment2D>()?;
    m.add_class::<LineSegment3D>()?;
    Ok(())
}
