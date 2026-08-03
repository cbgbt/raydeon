use std::sync::Arc;

use numpy::{Ix1, PyArray};
use pyo3::prelude::*;

use crate::material::Material;
use crate::shapes::Geometry;

#[derive(Debug)]
#[pyclass(frozen)]
/// A `DrawableShape` is the input geometry for a pyraydeon scene.
///
/// It is essentially some drawable geometry joined with a given material.
pub(crate) struct DrawableShape {
    pub raydeon_drawable: raydeon::DrawableShape,
    pub pyobj: PyObject,
}

impl ::std::ops::Deref for DrawableShape {
    type Target = raydeon::DrawableShape;

    fn deref(&self) -> &Self::Target {
        &self.raydeon_drawable
    }
}

impl DrawableShape {
    pub(crate) fn raydeon_drawable(&self) -> raydeon::DrawableShape {
        self.raydeon_drawable.clone()
    }
}

impl<'py> FromPyObject<'py> for DrawableShape {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = obj.py();
        let shape: PyObject = obj.getattr("shape")?.extract()?;
        let material: Option<Material> = obj.getattr("material")?.extract()?;

        let raydeon_geometry = raydeon_geometry_from_py_object(py, &shape)?;
        let raydeon_drawable = raydeon::DrawableShape::new()
            .geometry(raydeon_geometry)
            .maybe_material(material.map(|m| m.0))
            .build();

        Ok(DrawableShape {
            raydeon_drawable,
            pyobj: shape,
        })
    }
}

#[pymethods]
impl DrawableShape {
    #[new]
    #[pyo3(signature = (geometry, material=None))]
    fn new(py: Python, geometry: PyObject, material: Option<Material>) -> PyResult<Self> {
        let raydeon_geometry = raydeon_geometry_from_py_object(py, &geometry)?;
        let material = material.map(|m| m.0);
        let raydeon_drawable = raydeon::DrawableShape::new()
            .geometry(raydeon_geometry)
            .maybe_material(material)
            .build();

        Ok(DrawableShape {
            raydeon_drawable,
            pyobj: geometry,
        })
    }

    #[getter]
    fn material(&self) -> Option<Material> {
        self.raydeon_drawable.material().cloned().map(Into::into)
    }

    #[getter]
    fn shape(&self, py: Python) -> PyObject {
        self.pyobj.clone_ref(py)
    }

    fn collision_geometry(&self, py: Python) -> PyResult<PyObject> {
        self.pyobj.call_method0(py, "collision_geometry")
    }

    fn paths(&self, py: Python) -> PyResult<PyObject> {
        self.pyobj.call_method0(py, "paths")
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow()))
    }
}

pub(crate) fn raydeon_geometry_from_py_object(
    py: Python,
    g: &PyObject,
) -> PyResult<Arc<dyn raydeon::Shape>> {
    let geom: Py<Geometry> = g.extract(py)?;
    let raydeon_shape = geom.borrow(py);
    let raydeon_shape = raydeon_shape.geometry(g.clone_ref(py));
    Ok(raydeon_shape)
}

#[derive(Debug)]
#[pyclass(frozen)]
/// One pen stroke of a finished render.
pub(crate) struct Stroke {
    p1: [f64; 2],
    p2: [f64; 2],
    pen: usize,
    kind: StrokeKind,
}

impl From<&raydeon::Stroke> for Stroke {
    fn from(value: &raydeon::Stroke) -> Self {
        Self {
            p1: value.p1.to_array(),
            p2: value.p2.to_array(),
            pen: value.pen.value(),
            kind: value.kind.into(),
        }
    }
}

#[pymethods]
impl Stroke {
    #[getter]
    fn p1<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.p1)
    }

    #[getter]
    fn p2<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.p2)
    }

    #[getter]
    fn pen(&self) -> usize {
        self.pen
    }

    #[getter]
    fn kind(&self) -> StrokeKind {
        self.kind
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow()))
    }
}

#[pyclass(eq)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum StrokeKind {
    Outline,
    Hatch,
}

impl From<raydeon::StrokeKind> for StrokeKind {
    fn from(value: raydeon::StrokeKind) -> Self {
        match value {
            raydeon::StrokeKind::Outline => StrokeKind::Outline,
            raydeon::StrokeKind::Hatch => StrokeKind::Hatch,
        }
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DrawableShape>()?;
    m.add_class::<Stroke>()?;
    m.add_class::<StrokeKind>()?;
    Ok(())
}
