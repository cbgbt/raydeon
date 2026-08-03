use pyo3::prelude::*;

pywrap!(Material, raydeon::material::Material);

#[pymethods]
impl Material {
    #[new]
    #[pyo3(signature = (diffuse=0.0, specular=0.0, shininess=0.0, pen=0))]
    fn new(diffuse: f64, specular: f64, shininess: f64, pen: usize) -> PyResult<Self> {
        Ok(raydeon::material::Material::new()
            .diffuse(diffuse)
            .specular(specular)
            .shininess(shininess)
            .pen(raydeon::PenId::new(pen))
            .build()
            .into())
    }

    #[getter]
    fn diffuse(&self) -> f64 {
        self.diffuse
    }

    #[getter]
    fn specular(&self) -> f64 {
        self.specular
    }

    #[getter]
    fn shininess(&self) -> f64 {
        self.shininess
    }

    #[getter]
    fn pen(&self) -> usize {
        self.pen.value()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().0))
    }
}

impl Default for Material {
    fn default() -> Self {
        raydeon::material::Material::default().into()
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Material>()?;
    Ok(())
}
