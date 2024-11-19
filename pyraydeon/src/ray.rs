use numpy::PyArray1;
use pyo3::prelude::*;

use crate::linear::{Point3, Vec3};

pywrap!(Ray, raydeon::Ray);

#[pymethods]
impl Ray {
    #[new]
    fn new(point: &Bound<'_, PyAny>, dir: &Bound<'_, PyAny>) -> PyResult<Self> {
        let point = Point3::try_from(point)?;
        let dir = Vec3::try_from(dir)?;
        Ok(raydeon::Ray::new(point.0.cast_unit(), dir.0.cast_unit()).into())
    }

    #[getter]
    fn point<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let point = [self.0.point.x, self.0.point.y, self.0.point.z];
        PyArray1::from_slice_bound(py, &point)
    }

    #[getter]
    fn dir<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let dir = [self.0.dir.x, self.0.dir.y, self.0.dir.z];
        PyArray1::from_slice_bound(py, &dir)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pywrap!(HitData, raydeon::HitData);

#[pymethods]
impl HitData {
    #[new]
    fn new(hit_point: &Bound<'_, PyAny>, dist_to: f64) -> PyResult<Self> {
        let hit_point = Point3::try_from(hit_point)?;
        Ok(raydeon::HitData::new(hit_point.0.cast_unit(), dist_to).into())
    }

    #[getter]
    fn hit_point<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        let hp = [self.0.hit_point.x, self.0.hit_point.y, self.0.hit_point.z];
        PyArray1::from_slice_bound(py, &hp)
    }

    #[getter]
    fn dist_to(&self) -> f64 {
        self.0.dist_to
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Ray>()?;
    m.add_class::<HitData>()?;
    Ok(())
}
