use std::sync::Arc;

use numpy::{Ix1, PyArray, PyReadonlyArray1};
use pyo3::prelude::*;
use raydeon::WorldSpace;

use crate::light::PointLight;
use crate::linear::{ArbitrarySpace, Point2, Point3, Vec3};
use crate::material::Material;
use crate::shapes::Geometry;

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

    fn perspective(&self, fovy: f64, width: usize, height: usize, znear: f64, zfar: f64) -> Camera {
        self.0.perspective(fovy, width, height, znear, zfar).into()
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

#[pyclass(frozen)]
pub(crate) struct Scene {
    scene: Arc<
        raydeon::Scene<
            raydeon::scene::SceneGeometry<raydeon::material::Material>,
            raydeon::scene::SceneLighting,
        >,
    >,
}

#[pymethods]
impl Scene {
    #[new]
    #[pyo3(signature = (geometry=None, lights=None))]
    fn new(
        py: Python,
        geometry: Option<Vec<PyObject>>,
        lights: Option<Vec<PointLight>>,
    ) -> PyResult<Self> {
        let geometry = geometry.unwrap_or_default();
        let lights = lights.unwrap_or_default();
        let geometry: Vec<Arc<dyn raydeon::Shape<WorldSpace, raydeon::material::Material>>> =
            geometry
                .into_iter()
                .map(|g| {
                    let geom: Py<Geometry> = g.extract(py)?;
                    let raydeon_shape = geom.borrow(py);
                    let raydeon_shape = raydeon_shape.geometry(g);
                    Ok(raydeon_shape)
                })
                .collect::<PyResult<_>>()?;
        let lights: Vec<Arc<dyn raydeon::Light>> = lights
            .into_iter()
            .map(|l| Arc::new(l.0) as Arc<dyn raydeon::lights::Light>)
            .collect();
        let scene = Arc::new(
            raydeon::Scene::new()
                .with_geometry(geometry)
                .with_lighting(lights),
        );
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

    #[pyo3(signature = (camera, seed=None))]
    fn render_with_lighting(
        &self,
        py: Python,
        camera: &Camera,
        seed: Option<u64>,
    ) -> Vec<LineSegment2D> {
        py.allow_threads(|| {
            let cam = self.scene.attach_camera(camera.0);
            let cam = if let Some(seed) = seed {
                cam.with_seed(seed)
            } else {
                cam
            };
            let render_result = cam.render_with_lighting();
            render_result
                .geometry_paths
                .into_iter()
                .chain(render_result.hatch_paths)
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

pywrap!(LineSegment3D, raydeon::path::LineSegment3D<ArbitrarySpace, raydeon::material::Material>);

#[pymethods]
impl LineSegment3D {
    #[new]
    #[pyo3(signature = (p1, p2, material=None))]
    fn new(
        p1: &Bound<'_, PyAny>,
        p2: &Bound<'_, PyAny>,
        material: Option<Material>,
    ) -> PyResult<Self> {
        let p1 = Point3::try_from(p1)?;
        let p2 = Point3::try_from(p2)?;
        Ok(raydeon::path::LineSegment3D::tagged(
            p1.cast_unit(),
            p2.cast_unit(),
            material.unwrap_or_default().0,
        )
        .into())
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
