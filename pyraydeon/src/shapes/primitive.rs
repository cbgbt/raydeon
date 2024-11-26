use super::{CollisionGeometry, Geometry};
use crate::linear::{Point3, Vec3};
use crate::material::Material;
use numpy::{Ix1, PyArray, PyArrayLike1, PyArrayLike2};
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use std::sync::Arc;

#[pyclass(frozen, extends=Geometry, subclass)]
pub(crate) struct AxisAlignedCuboid(pub(crate) Arc<raydeon::shapes::AxisAlignedCuboid>);

impl ::std::ops::Deref for AxisAlignedCuboid {
    type Target = Arc<raydeon::shapes::AxisAlignedCuboid>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::AxisAlignedCuboid>> for AxisAlignedCuboid {
    fn from(value: Arc<raydeon::shapes::AxisAlignedCuboid>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl AxisAlignedCuboid {
    #[new]
    #[pyo3(signature = (min, max, material=None))]
    fn new(
        min: &Bound<'_, PyAny>,
        max: &Bound<'_, PyAny>,
        material: Option<Material>,
    ) -> PyResult<(Self, Geometry)> {
        let min: Vec3 = min.try_into()?;
        let max: Vec3 = max.try_into()?;

        let shape = Arc::new(
            raydeon::shapes::AxisAlignedCuboid::new()
                .min(min.cast_unit())
                .max(max.cast_unit())
                .material(material.map(|m| m.0).unwrap_or_default())
                .build(),
        );
        let geom = Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape>);

        Ok((Self(shape), geom))
    }
}

#[pyclass(frozen, extends=Geometry, subclass)]
pub(crate) struct Tri(pub(crate) Arc<raydeon::shapes::Triangle>);

impl ::std::ops::Deref for Tri {
    type Target = Arc<raydeon::shapes::Triangle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::Triangle>> for Tri {
    fn from(value: Arc<raydeon::shapes::Triangle>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Tri {
    #[new]
    #[pyo3(signature = (p1, p2, p3, material=None))]
    fn new(
        p1: &Bound<'_, PyAny>,
        p2: &Bound<'_, PyAny>,
        p3: &Bound<'_, PyAny>,
        material: Option<Material>,
    ) -> PyResult<(Self, Geometry)> {
        let p1: Point3 = p1.try_into()?;
        let p2: Point3 = p2.try_into()?;
        let p3: Point3 = p3.try_into()?;

        let shape = Arc::new(
            raydeon::shapes::Triangle::new()
                .v0(p1.cast_unit())
                .v1(p2.cast_unit())
                .v2(p3.cast_unit())
                .material(material.map(|m| m.0).unwrap_or_default())
                .build(),
        );
        let geom = Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape>);
        Ok((Self(shape), geom))
    }
}

#[pyclass(frozen, extends=CollisionGeometry, subclass)]
pub(crate) struct Plane(pub(crate) Arc<raydeon::shapes::Plane>);

impl ::std::ops::Deref for Plane {
    type Target = Arc<raydeon::shapes::Plane>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::Plane>> for Plane {
    fn from(value: Arc<raydeon::shapes::Plane>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Plane {
    #[new]
    fn new(
        point: &Bound<'_, PyAny>,
        normal: &Bound<'_, PyAny>,
    ) -> PyResult<(Self, CollisionGeometry)> {
        let point: Point3 = point.try_into()?;
        let normal: Vec3 = normal.try_into()?;

        let shape = Arc::new(
            raydeon::shapes::Plane::new()
                .point(point.0.cast_unit())
                .normal(normal.0.cast_unit())
                .build(),
        );
        let geom =
            CollisionGeometry::native(Arc::clone(&shape) as Arc<dyn raydeon::CollisionGeometry>);
        Ok((Self(shape), geom))
    }

    #[getter]
    fn point<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.point.to_array())
    }

    #[getter]
    fn normal<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.normal.to_array())
    }
}

#[pyclass(frozen, extends=CollisionGeometry, subclass)]
pub(crate) struct Sphere(pub(crate) Arc<raydeon::shapes::Sphere>);

impl ::std::ops::Deref for Sphere {
    type Target = Arc<raydeon::shapes::Sphere>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::Sphere>> for Sphere {
    fn from(value: Arc<raydeon::shapes::Sphere>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Sphere {
    #[new]
    fn new(center: &Bound<'_, PyAny>, radius: f64) -> PyResult<(Self, CollisionGeometry)> {
        let center: Point3 = center.try_into()?;

        let shape = Arc::new(
            raydeon::shapes::Sphere::new()
                .center(center.0.cast_unit())
                .radius(radius)
                .build(),
        );
        let geom =
            CollisionGeometry::native(Arc::clone(&shape) as Arc<dyn raydeon::CollisionGeometry>);
        Ok((Self(shape), geom))
    }

    #[getter]
    fn center<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.center.to_array())
    }

    #[getter]
    fn radius(&self) -> f64 {
        self.0.radius
    }
}

#[pyclass(frozen, extends=Geometry, subclass)]
pub(crate) struct Quad(pub(crate) Arc<raydeon::shapes::Quad>);

impl ::std::ops::Deref for Quad {
    type Target = Arc<raydeon::shapes::Quad>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::Quad>> for Quad {
    fn from(value: Arc<raydeon::shapes::Quad>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Quad {
    #[new]
    #[pyo3(signature = (origin, basis, dims, material=None))]
    fn new(
        origin: &Bound<'_, PyAny>,
        basis: PyArrayLike2<'_, f64>,
        dims: PyArrayLike1<'_, f64>,
        material: Option<Material>,
    ) -> PyResult<(Self, Geometry)> {
        let origin: Point3 = origin.try_into()?;
        let basis = basis
            .as_array()
            .as_slice()
            .ok_or(PyIndexError::new_err("basis must be 2x3 array"))
            .and_then(|slice| {
                if slice.len() != 6 {
                    Err(PyIndexError::new_err("basis must be 2x3 array"))
                } else {
                    Ok(slice)
                }
            })
            .map(|basis| {
                [
                    raydeon::Vec3::new(basis[0], basis[1], basis[2]),
                    raydeon::Vec3::new(basis[3], basis[4], basis[5]),
                ]
            })?;
        let dims = dims
            .as_array()
            .as_slice()
            .ok_or(PyIndexError::new_err("dims must be 1x2 array"))
            .and_then(|slice| {
                if slice.len() != 2 {
                    Err(PyIndexError::new_err("dims must be 1x2 array"))
                } else {
                    Ok(slice)
                }
            })
            .map(|dims| [dims[0], dims[1]])?;

        let shape = Arc::new(
            raydeon::shapes::Quad::new()
                .origin(origin.0.cast_unit())
                .basis(basis)
                .dims(dims)
                .material(material.map(|m| m.0).unwrap_or_default())
                .build(),
        );
        let geom = Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape>);
        Ok((Self(shape), geom))
    }
}
