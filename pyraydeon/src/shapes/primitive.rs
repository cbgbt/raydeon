use super::{CollisionGeometry, Geometry};
use crate::linear::{Point3, Vec3};
use pyo3::prelude::*;
use raydeon::WorldSpace;
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
    #[pyo3(signature = (min, max, tag=0))]
    fn new(
        min: &Bound<'_, PyAny>,
        max: &Bound<'_, PyAny>,
        tag: usize,
    ) -> PyResult<(Self, Geometry)> {
        let min: Vec3 = min.try_into()?;
        let max: Vec3 = max.try_into()?;

        let shape = Arc::new(raydeon::shapes::AxisAlignedCuboid::tagged(
            min.cast_unit(),
            max.cast_unit(),
            tag,
        ));
        let geom = Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape<WorldSpace>>);

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
    #[pyo3(signature = (p1, p2, p3, tag=0))]
    fn new(
        p1: &Bound<'_, PyAny>,
        p2: &Bound<'_, PyAny>,
        p3: &Bound<'_, PyAny>,
        tag: usize,
    ) -> PyResult<(Self, Geometry)> {
        let p1: Point3 = p1.try_into()?;
        let p2: Point3 = p2.try_into()?;
        let p3: Point3 = p3.try_into()?;

        let shape = Arc::new(raydeon::shapes::Triangle::tagged(
            p1.cast_unit(),
            p2.cast_unit(),
            p3.cast_unit(),
            tag,
        ));
        let geom = Geometry::native(Arc::clone(&shape) as Arc<dyn raydeon::Shape<WorldSpace>>);
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
