use pyo3::prelude::*;

use crate::hatch::{ContourStyle, HatchStyle};

pywrap!(PenId, raydeon::PenId);

#[pymethods]
impl PenId {
    /// The pen which draws a material's strokes, numbered from zero.
    #[new]
    fn new(value: usize) -> Self {
        raydeon::PenId::new(value).into()
    }

    #[getter]
    fn value(&self) -> usize {
        self.0.value()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> usize {
        self.0.value()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

/// A pen named either by its number or by identity.
#[derive(FromPyObject)]
enum Pen {
    Id(PenId),
    Number(usize),
}

impl Pen {
    fn parse(self) -> raydeon::PenId {
        match self {
            Pen::Id(pen) => pen.0,
            Pen::Number(value) => raydeon::PenId::new(value),
        }
    }
}

pywrap!(Material, raydeon::material::Material);

#[pymethods]
impl Material {
    /// How a surface takes light, and how it draws itself.
    ///
    /// An omitted `pen` plots with pen zero, an omitted `hatch` draws
    /// outlines only, and an omitted `contours` draws no iso-contours.
    #[new]
    #[pyo3(signature = (diffuse=0.0, specular=0.0, shininess=0.0, pen=None, hatch=None, contours=None))]
    fn new(
        diffuse: f64,
        specular: f64,
        shininess: f64,
        pen: Option<Pen>,
        hatch: Option<HatchStyle>,
        contours: Option<ContourStyle>,
    ) -> PyResult<Self> {
        Ok(raydeon::material::Material::new()
            .diffuse(diffuse)
            .specular(specular)
            .shininess(shininess)
            .pen(pen.map(Pen::parse).unwrap_or_default())
            .maybe_hatch(hatch.map(|style| style.0))
            .maybe_contours(contours.map(|style| style.0))
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
    fn pen(&self) -> PenId {
        self.pen.into()
    }

    #[getter]
    fn hatch(&self) -> Option<HatchStyle> {
        self.hatch.clone().map(Into::into)
    }

    #[getter]
    fn contours(&self) -> Option<ContourStyle> {
        self.contours.clone().map(Into::into)
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
    m.add_class::<PenId>()?;
    m.add_class::<Material>()?;
    Ok(())
}
