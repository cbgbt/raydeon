use super::{CollisionGeometry, Geometry};
use crate::linear::{Point3, Vec3};
use crate::Material;
use numpy::{PyArrayLike1, PyArrayLike2};
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use raydeon::WorldSpace;
use std::sync::Arc;

#[pyclass(frozen, extends=Geometry, subclass)]
pub(crate) struct AxisAlignedCuboid(pub(crate) Arc<raydeon::shapes::AxisAlignedCuboid<Material>>);

impl ::std::ops::Deref for AxisAlignedCuboid {
    type Target = Arc<raydeon::shapes::AxisAlignedCuboid<Material>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::AxisAlignedCuboid<Material>>> for AxisAlignedCuboid {
    fn from(value: Arc<raydeon::shapes::AxisAlignedCuboid<Material>>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl AxisAlignedCuboid {
    #[new]
    #[pyo3(signature = (min, max))]
    fn new(min: &Bound<'_, PyAny>, max: &Bound<'_, PyAny>) -> PyResult<(Self, Geometry)> {
        let min: Vec3 = min.try_into()?;
        let max: Vec3 = max.try_into()?;

        let shape = Arc::new(raydeon::shapes::AxisAlignedCuboid::tagged(
            min.cast_unit(),
            max.cast_unit(),
            Material,
        ));
        let geom =
            Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape<WorldSpace, Material>>);

        Ok((Self(shape), geom))
    }
}

#[pyclass(frozen, extends=Geometry, subclass)]
pub(crate) struct Tri(pub(crate) Arc<raydeon::shapes::Triangle<Material>>);

impl ::std::ops::Deref for Tri {
    type Target = Arc<raydeon::shapes::Triangle<Material>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::Triangle<Material>>> for Tri {
    fn from(value: Arc<raydeon::shapes::Triangle<Material>>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Tri {
    #[new]
    #[pyo3(signature = (p1, p2, p3))]
    fn new(
        p1: &Bound<'_, PyAny>,
        p2: &Bound<'_, PyAny>,
        p3: &Bound<'_, PyAny>,
    ) -> PyResult<(Self, Geometry)> {
        let p1: Point3 = p1.try_into()?;
        let p2: Point3 = p2.try_into()?;
        let p3: Point3 = p3.try_into()?;

        let shape = Arc::new(raydeon::shapes::Triangle::tagged(
            p1.cast_unit(),
            p2.cast_unit(),
            p3.cast_unit(),
            Material,
        ));
        let geom =
            Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape<WorldSpace, Material>>);
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

        let shape = Arc::new(raydeon::shapes::Plane::new(
            point.0.cast_unit(),
            normal.0.cast_unit(),
        ));
        let geom = CollisionGeometry::native(
            Arc::clone(&shape) as Arc<dyn raydeon::CollisionGeometry<WorldSpace>>
        );
        Ok((Self(shape), geom))
    }
}

#[pyclass(frozen, extends=Geometry, subclass)]
pub(crate) struct Quad(pub(crate) Arc<raydeon::shapes::Quad<Material>>);

impl ::std::ops::Deref for Quad {
    type Target = Arc<raydeon::shapes::Quad<Material>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<raydeon::shapes::Quad<Material>>> for Quad {
    fn from(value: Arc<raydeon::shapes::Quad<Material>>) -> Self {
        Self(value)
    }
}

#[pymethods]
impl Quad {
    #[new]
    #[pyo3(signature = (origin, basis, dims))]
    fn new(
        origin: &Bound<'_, PyAny>,
        basis: PyArrayLike2<'_, f64>,
        dims: PyArrayLike1<'_, f64>,
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

        let shape = Arc::new(raydeon::shapes::Quad::tagged(
            origin.0.cast_unit(),
            basis,
            dims,
            Material,
        ));
        let geom =
            Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape<WorldSpace, Material>>);
        Ok((Self(shape), geom))
    }
}
