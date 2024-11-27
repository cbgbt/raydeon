use std::sync::Arc;

use numpy::{Ix1, PyArray, PyReadonlyArray1};
use pyo3::prelude::*;
use raydeon::SceneLighting;

use crate::camera::Camera;
use crate::light::PointLight;
use crate::linear::{ArbitrarySpace, Point2, Point3};
use crate::material::Material;
use crate::shapes::Geometry;

#[pyclass(frozen)]
pub(crate) struct Scene {
    scene: Arc<raydeon::Scene>,
}

#[pymethods]
impl Scene {
    #[new]
    #[pyo3(signature = (geometry=None, lights=None, ambient_light=0.0))]
    fn new(
        py: Python,
        geometry: Option<Vec<PyObject>>,
        lights: Option<Vec<PointLight>>,
        ambient_light: f64,
    ) -> PyResult<Self> {
        let geometry = geometry.unwrap_or_default();
        let lights = lights.unwrap_or_default();
        let geometry: Vec<Arc<dyn raydeon::Shape>> = geometry
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
        let lighting = SceneLighting::new()
            .with_lights(lights)
            .with_ambient_lighting(ambient_light);
        let scene = Arc::new(
            raydeon::Scene::new()
                .geometry(geometry)
                .lighting(lighting)
                .construct(),
        );
        Ok(Self { scene })
    }

    fn render(&self, py: Python, camera: &Camera) -> Vec<LineSegment2D> {
        py.allow_threads(|| {
            let cam = self.scene.attach_camera(camera.0.clone());
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
            let cam = self.scene.attach_camera(camera.0.clone());
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

pywrap!(LineSegment3D, raydeon::path::LineSegment3D<ArbitrarySpace>);

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
        Ok(raydeon::path::LineSegment3D::new()
            .p1(p1.cast_unit())
            .p2(p2.cast_unit())
            .maybe_material(material.map(|i| i.0))
            .build()
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
    m.add_class::<Scene>()?;
    m.add_class::<LineSegment2D>()?;
    m.add_class::<LineSegment3D>()?;
    Ok(())
}
