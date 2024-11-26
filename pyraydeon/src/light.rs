use crate::linear::Point3;
use pyo3::prelude::*;

pywrap!(PointLight, raydeon::lights::PointLight);

#[pymethods]
impl PointLight {
    #[new]
    #[pyo3(signature = (
        position,
        intensity=0.0,
        specular_intensity=0.0,
        constant_attenuation=0.0,
        linear_attenuation=0.0,
        quadratic_attenuation=0.0
    ))]
    fn new(
        position: &Bound<'_, PyAny>,
        intensity: f64,
        specular_intensity: f64,
        constant_attenuation: f64,
        linear_attenuation: f64,
        quadratic_attenuation: f64,
    ) -> PyResult<Self> {
        let position = Point3::try_from(position)?;
        Ok(raydeon::lights::PointLight::new(
            intensity,
            specular_intensity,
            position.0.cast_unit(),
            constant_attenuation,
            linear_attenuation,
            quadratic_attenuation,
        )
        .into())
    }

    #[getter]
    fn intensity(&self) -> f64 {
        self.0.intensity()
    }

    #[getter]
    fn specular(&self) -> f64 {
        self.0.specular()
    }

    #[getter]
    fn position(&self) -> Point3 {
        self.0.position().cast_unit().into()
    }

    #[getter]
    fn constant_attenuation(&self) -> f64 {
        self.0.constant_attenuation()
    }

    #[getter]
    fn linear_attenuation(&self) -> f64 {
        self.0.linear_attenuation()
    }

    #[getter]
    fn quadratic_attenuation(&self) -> f64 {
        self.0.quadratic_attenuation()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().0))
    }
}

impl Default for PointLight {
    fn default() -> Self {
        raydeon::lights::PointLight::default().into()
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PointLight>()?;
    Ok(())
}
