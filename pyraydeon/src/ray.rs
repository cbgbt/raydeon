use numpy::{Ix1, PyArray, PyReadonlyArray1};
use pyo3::prelude::*;

use crate::linear::{Point3, Vec3};

pywrap!(Ray, raydeon::Ray);

#[pymethods]
impl Ray {
    #[new]
    fn new(point: PyReadonlyArray1<f64>, dir: PyReadonlyArray1<f64>) -> PyResult<Self> {
        let point = Point3::try_from(point)?;
        let dir = Vec3::try_from(dir)?;
        Ok(raydeon::Ray::new(point.0.cast_unit(), dir.0.cast_unit()).into())
    }

    #[getter]
    fn point<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.point.to_array())
    }

    #[getter]
    fn dir<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.dir.to_array())
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
    fn new(hit_point: PyReadonlyArray1<f64>, dist_to: f64) -> PyResult<Self> {
        let hit_point = Point3::try_from(hit_point)?;
        Ok(raydeon::HitData::new(hit_point.0.cast_unit(), dist_to).into())
    }

    #[getter]
    fn hit_point<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.hit_point.to_array())
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
