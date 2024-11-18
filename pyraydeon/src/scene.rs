use std::sync::Arc;

use pyo3::prelude::*;
use raydeon::WorldSpace;

use crate::linear::{ArbitrarySpace, Point2, Point3, Vec3};
use crate::shapes::Geometry;

pywrap!(Camera, raydeon::Camera);

#[pymethods]
impl Camera {
    #[staticmethod]
    fn look_at(eye: &Point3, center: &Vec3, up: &Vec3) -> LookingCamera {
        raydeon::Camera::look_at(eye.cast_unit(), center.cast_unit(), up.cast_unit()).into()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pywrap!(LookingCamera, raydeon::scene::LookingCamera);

#[pymethods]
impl LookingCamera {
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
    scene: Arc<raydeon::Scene>,
}

#[pymethods]
impl Scene {
    #[new]
    fn new(py: Python, geometry: Vec<PyObject>) -> PyResult<Self> {
        let geometry: Vec<Arc<dyn raydeon::Shape<WorldSpace>>> = geometry
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
    #[getter]
    fn p1(&self) -> Point2 {
        self.p1.cast_unit().into()
    }

    #[getter]
    fn p2(&self) -> Point2 {
        self.p2.cast_unit().into()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pywrap!(LineSegment3D, raydeon::path::LineSegment3D<ArbitrarySpace>);

#[pymethods]
impl LineSegment3D {
    #[getter]
    fn p1(&self) -> Point3 {
        self.p1.cast_unit().into()
    }

    #[getter]
    fn p2(&self) -> Point3 {
        self.p2.cast_unit().into()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Camera>()?;
    // `LookingCamera` remains "private"
    m.add_class::<Scene>()?;
    Ok(())
}
