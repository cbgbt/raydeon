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
        self.raydeon_drawable.material().map(Into::into)
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
/// The output of a raydeon render.
///
/// A line segment, associated with a shape.
pub(crate) struct DrawableSegment {
    p1: [f64; 2],
    p2: [f64; 2],
    kind: SegmentKind,

    raydeon_drawable: Option<raydeon::DrawableShape>,
}

impl From<raydeon::DrawableSegment<'_>> for DrawableSegment {
    fn from(value: raydeon::DrawableSegment<'_>) -> Self {
        let p1 = value.p1.to_array();
        let p2 = value.p2.to_array();

        let raydeon_drawable = value.segment.get_shape().cloned();

        let kind = value.kind.into();

        Self {
            p1,
            p2,
            raydeon_drawable,
            kind,
        }
    }
}

#[pymethods]
impl DrawableSegment {
    #[getter]
    fn p1<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.p1)
    }

    #[getter]
    fn p2<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.p2)
    }

    #[getter]
    fn material(&self) -> Option<Material> {
        self.raydeon_drawable
            .as_ref()
            .and_then(|d| d.material())
            .map(|m| m.into())
    }

    #[getter]
    fn kind(&self) -> SegmentKind {
        self.kind
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow()))
    }
}

#[pyclass(eq)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum SegmentKind {
    VerticalHatch,
    DiagonalHatch,
    Path,
}

impl From<raydeon::SegmentKind> for SegmentKind {
    fn from(value: raydeon::SegmentKind) -> Self {
        match value {
            raydeon::SegmentKind::ScreenSpaceHatch(raydeon::ScreenSpaceHatchKind::Vertical) => {
                SegmentKind::VerticalHatch
            }
            raydeon::SegmentKind::ScreenSpaceHatch(raydeon::ScreenSpaceHatchKind::Diagonal60) => {
                SegmentKind::VerticalHatch
            }
            raydeon::SegmentKind::Path => SegmentKind::Path,
        }
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DrawableShape>()?;
    m.add_class::<DrawableSegment>()?;
    Ok(())
}
