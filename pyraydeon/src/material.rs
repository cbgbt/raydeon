use pyo3::prelude::*;

pywrap!(Material, raydeon::material::Material);

#[pymethods]
impl Material {
    #[new]
    #[pyo3(signature = (diffuse=0.0, specular=0.0, shininess=0.0, tag=0))]
    fn new(diffuse: f64, specular: f64, shininess: f64, tag: usize) -> PyResult<Self> {
        Ok(raydeon::material::Material::new(diffuse, specular, shininess, tag).into())
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
    fn tag(&self) -> usize {
        self.tag
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
