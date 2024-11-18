use pyo3::prelude::*;

use crate::linear::{Point3, Vec3};

pywrap!(Ray, raydeon::Ray);

#[pymethods]
impl Ray {
    #[new]
    fn new(point: &Point3, dir: &Vec3) -> Self {
        raydeon::Ray::new(point.0.cast_unit(), dir.0.cast_unit()).into()
    }

    #[getter]
    fn point(&self) -> Point3 {
        self.0.point.cast_unit().into()
    }

    #[getter]
    fn dir(&self) -> Vec3 {
        self.0.dir.cast_unit().into()
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
    fn new(hit_point: &Point3, dist_to: f64) -> Self {
        raydeon::HitData::new(hit_point.0.cast_unit(), dist_to).into()
    }

    #[getter]
    fn hit_point(&self) -> Point3 {
        self.0.hit_point.cast_unit().into()
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
