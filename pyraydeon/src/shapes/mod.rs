use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use raydeon::WorldSpace;
use std::sync::Arc;

mod primitive;

pub(crate) use primitive::{AxisAlignedCuboid, Tri};

use crate::linear::AABB3;
use crate::ray::{HitData, Ray};
use crate::scene::{Camera, LineSegment3D};

#[derive(Debug)]
enum InnerGeometry {
    Native(Arc<dyn raydeon::Shape<WorldSpace>>),
    Py,
}

#[derive(Debug)]
#[pyclass(subclass, frozen)]
pub(crate) struct Geometry {
    geom: InnerGeometry,
}

impl Geometry {
    pub(crate) fn native(geom: Arc<dyn raydeon::Shape<WorldSpace>>) -> Self {
        let geom = InnerGeometry::Native(geom);
        Self { geom }
    }

    pub(crate) fn py() -> Self {
        let geom = InnerGeometry::Py;
        Self { geom }
    }

    pub(crate) fn geometry(&self, obj: PyObject) -> Arc<dyn raydeon::Shape<WorldSpace>> {
        match &self.geom {
            InnerGeometry::Native(ref geom) => Arc::clone(geom),
            InnerGeometry::Py => Arc::new(PythonGeometry { slf: obj }),
        }
    }
}

#[pymethods]
impl Geometry {
    #[new]
    #[pyo3(signature = (*_py_args, **_py_kwargs))]
    fn new(_py_args: &Bound<'_, PyTuple>, _py_kwargs: Option<&Bound<'_, PyDict>>) -> Self {
        Self::py()
    }

    fn hit_by(&self, ray: &Ray) -> Option<HitData> {
        match self.geom {
            InnerGeometry::Native(ref geom) => geom.hit_by(&ray.0).map(Into::into),
            InnerGeometry::Py => None,
        }
    }

    fn paths(&self, cam: &Camera) -> Vec<LineSegment3D> {
        match &self.geom {
            InnerGeometry::Native(geom) => {
                let paths = geom.paths(&cam.0);
                paths
                    .into_iter()
                    .map(raydeon::path::LineSegment3D::cast_unit)
                    .map(Into::into)
                    .collect()
            }
            InnerGeometry::Py => Vec::new(),
        }
    }

    fn bounding_box(&self) -> Option<AABB3> {
        match &self.geom {
            InnerGeometry::Native(geom) => geom
                .bounding_box()
                .as_ref()
                .map(raydeon::AABB3::cast_unit)
                .map(Into::into),
            InnerGeometry::Py => None,
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().geom))
    }
}

#[derive(Debug)]
struct PythonGeometry {
    slf: PyObject,
}

impl raydeon::Shape<WorldSpace> for PythonGeometry {
    fn hit_by(&self, ray: &raydeon::Ray) -> Option<raydeon::HitData> {
        Python::with_gil(|py| {
            let inner = self.slf.bind_borrowed(py);
            let ray = Ray::from(*ray);
            let call_result = inner.call_method1("hit_by", (ray,)).ok()?;

            call_result
                .extract::<Option<HitData>>()
                .ok()?
                .map(|hit| hit.0)
        })
    }

    fn paths(&self, cam: &raydeon::Camera) -> Vec<raydeon::path::LineSegment3D<WorldSpace>> {
        let segments: Option<_> = Python::with_gil(|py| {
            let inner = self.slf.bind_borrowed(py);
            let cam = Camera::from(*cam);
            let call_result = inner.call_method1("paths", (cam,)).ok()?;

            let segments = call_result.extract::<Vec<LineSegment3D>>().ok()?;

            Some(
                segments
                    .into_iter()
                    .map(|segment| segment.0.cast_unit())
                    .collect(),
            )
        });
        segments.unwrap_or_default()
    }

    fn bounding_box(&self) -> Option<raydeon::AABB3<WorldSpace>> {
        Python::with_gil(|py| {
            let inner = self.slf.bind_borrowed(py);
            let call_result = inner.call_method1("hit_by", ()).ok()?;

            call_result
                .extract::<Option<AABB3>>()
                .ok()?
                .map(|aabb| aabb.0.cast_unit())
        })
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AxisAlignedCuboid>()?;
    m.add_class::<Tri>()?;
    m.add_class::<Geometry>()?;
    Ok(())
}
