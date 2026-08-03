use primitive::{Plane, Quad, Sphere};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use raydeon::WorldSpace;
use std::sync::Arc;

mod primitive;

pub(crate) use primitive::{AxisAlignedCuboid, Tri};

use crate::camera::Camera;
use crate::drawables::{raydeon_geometry_from_py_object, DrawableShape};
use crate::hatch::{hatch_surface_from_py, hatch_surface_into_py};
use crate::material::Material;
use crate::ray::{HitData, Ray, AABB3};
use crate::scene::LineSegment3D;

#[derive(Debug)]
enum InnerGeometry {
    Native(Arc<dyn raydeon::Shape>),
    Py,
}

#[derive(Debug)]
#[pyclass(subclass, frozen)]
pub(crate) struct Geometry {
    geom: InnerGeometry,
}

impl Geometry {
    pub(crate) fn native(geom: Arc<dyn raydeon::Shape>) -> Self {
        let geom = InnerGeometry::Native(geom);
        Self { geom }
    }

    pub(crate) fn py() -> Self {
        let geom = InnerGeometry::Py;
        Self { geom }
    }

    pub(crate) fn geometry(&self, obj: PyObject) -> Arc<dyn raydeon::Shape> {
        match &self.geom {
            InnerGeometry::Native(ref geom) => Arc::clone(geom),
            InnerGeometry::Py => Arc::new(PythonGeometry::new(obj, PythonGeometryKind::Draw)),
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

    fn collision_geometry(&self, py: Python) -> Option<Vec<CollisionGeometry>> {
        match &self.geom {
            InnerGeometry::Native(geom) => py.allow_threads(|| {
                let geometry = geom.collision_geometry()?;
                Some(
                    geometry
                        .into_iter()
                        .map(CollisionGeometry::native)
                        .collect(),
                )
            }),
            InnerGeometry::Py => None,
        }
    }

    fn paths(&self, py: Python, cam: &Camera) -> Vec<LineSegment3D> {
        match &self.geom {
            InnerGeometry::Native(geom) => py.allow_threads(|| {
                let paths = geom.paths(&cam.0);
                paths.into_iter().map(Into::into).collect()
            }),
            InnerGeometry::Py => Vec::new(),
        }
    }

    /// The surfaces this shape offers up for world-space hatching.
    ///
    /// A shape written in Python offers none until it says otherwise, so
    /// hatching is something a Python shape opts in to by overriding this.
    fn hatch_surfaces(&self, py: Python) -> Vec<PyObject> {
        match &self.geom {
            InnerGeometry::Native(geom) => geom
                .hatch_surfaces()
                .into_iter()
                .map(|surface| hatch_surface_into_py(py, surface))
                .collect(),
            InnerGeometry::Py => Vec::new(),
        }
    }

    fn with_material(slf: &Bound<'_, Self>, mat: Material, py: Python) -> PyResult<DrawableShape> {
        let obj_ptr = slf.as_any().as_unbound().clone_ref(py);
        let radeon_geom = raydeon_geometry_from_py_object(py, &obj_ptr)?;

        let raydeon_drawable = raydeon::DrawableShape::new()
            .geometry(radeon_geom)
            .material(mat.0)
            .build();

        Ok(DrawableShape {
            raydeon_drawable,
            pyobj: obj_ptr,
        })
    }

    #[getter]
    fn shape(slf: &Bound<'_, Self>, py: Python) -> PyObject {
        slf.as_any().as_unbound().clone_ref(py)
    }

    #[getter]
    fn material(&self) -> Option<Material> {
        None
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow()))
    }
}

#[derive(Debug)]
enum InnerCollisionGeometry {
    Native(Arc<dyn raydeon::CollisionGeometry>),
    Py,
}

#[derive(Debug)]
#[pyclass(subclass, frozen)]
pub(crate) struct CollisionGeometry {
    geom: InnerCollisionGeometry,
}

impl CollisionGeometry {
    pub(crate) fn native(geom: Arc<dyn raydeon::CollisionGeometry>) -> Self {
        let geom = InnerCollisionGeometry::Native(geom);
        Self { geom }
    }

    pub(crate) fn py() -> Self {
        let geom = InnerCollisionGeometry::Py;
        Self { geom }
    }
}

#[pymethods]
impl CollisionGeometry {
    #[new]
    #[pyo3(signature = (*_py_args, **_py_kwargs))]
    fn new(_py_args: &Bound<'_, PyTuple>, _py_kwargs: Option<&Bound<'_, PyDict>>) -> Self {
        Self::py()
    }

    fn hit_by(&self, py: Python, ray: &Ray) -> Option<HitData> {
        match self.geom {
            InnerCollisionGeometry::Native(ref geom) => {
                py.allow_threads(|| geom.hit_by(&ray.0).map(Into::into))
            }
            InnerCollisionGeometry::Py => None,
        }
    }

    fn bounding_box(&self, py: Python) -> Option<AABB3> {
        match &self.geom {
            InnerCollisionGeometry::Native(geom) => py.allow_threads(|| {
                geom.bounding_box()
                    .as_ref()
                    .map(raydeon::AABB3::cast_unit)
                    .map(Into::into)
            }),
            InnerCollisionGeometry::Py => None,
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().geom))
    }
}

#[derive(Debug)]
struct PythonGeometry {
    slf: PyObject,
    kind: PythonGeometryKind,
}

#[derive(Debug)]
enum PythonGeometryKind {
    Draw,
    Collision { aabb: Option<AABB3> },
}

impl PythonGeometry {
    fn new(slf: PyObject, kind: PythonGeometryKind) -> Self {
        Self { slf, kind }
    }

    fn as_collision_geometry(slf: PyObject) -> Self {
        let mut ret = Self {
            slf,
            kind: PythonGeometryKind::Draw,
        };
        let aabb = raydeon::CollisionGeometry::bounding_box(&ret);
        let aabb = aabb.map(|aabb| aabb.cast_unit().into());
        ret.kind = PythonGeometryKind::Collision { aabb };
        ret
    }
}

/// What a Python shape answered, or nothing at all.
///
/// A shape which cannot answer a question about itself has its complaint
/// handed to Python's unraisable hook — the render has no caller to fail
/// back to — and the answer is read as an absence.
fn answered<T>(py: Python<'_>, obj: &Bound<'_, PyAny>, result: PyResult<T>) -> Option<T> {
    match result {
        Ok(answer) => Some(answer),
        Err(err) => {
            err.write_unraisable_bound(py, Some(obj));
            None
        }
    }
}

impl raydeon::Shape for PythonGeometry {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn raydeon::CollisionGeometry>>> {
        Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let call_result = inner.call_method0("collision_geometry").ok()?;

            let nullable = answered(py, inner, call_result.extract::<Option<Bound<'_, PyAny>>>())?;
            let collision_iter = answered(py, inner, nullable?.iter())?;

            let geometry = collision_iter
                .map(|obj| {
                    Ok(
                        Arc::new(PythonGeometry::as_collision_geometry(obj?.into_py(py)))
                            as Arc<dyn raydeon::CollisionGeometry>,
                    )
                })
                .collect::<PyResult<Vec<_>>>();
            answered(py, inner, geometry)
        })
    }

    fn hatch_surfaces(&self) -> Vec<raydeon::HatchSurface> {
        Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let surfaces = inner
                .call_method0("hatch_surfaces")
                .and_then(|call_result| call_result.extract::<Option<Vec<Bound<'_, PyAny>>>>())
                .and_then(|offered| {
                    offered
                        .unwrap_or_default()
                        .iter()
                        .map(hatch_surface_from_py)
                        .collect::<PyResult<Vec<_>>>()
                });
            answered(py, inner, surfaces).unwrap_or_default()
        })
    }

    fn paths(&self, cam: &raydeon::Camera) -> Vec<raydeon::path::LineSegment3D<WorldSpace>> {
        Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let cam = Camera::from(cam.clone());
            let segments = inner
                .call_method1("paths", (cam,))
                .and_then(|call_result| call_result.extract::<Option<Vec<LineSegment3D>>>());

            answered(py, inner, segments)
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect()
        })
    }
}

impl raydeon::CollisionGeometry for PythonGeometry {
    fn hit_by(&self, ray: &raydeon::Ray) -> Option<raydeon::HitData> {
        if let PythonGeometryKind::Collision { aabb: Some(aabb) } = &self.kind {
            raydeon::shapes::AxisAlignedCuboid::from(aabb.0.cast_unit()).hit_by(ray)?;
        }
        Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let ray = Ray::from(*ray);
            let hit = inner
                .call_method1("hit_by", (ray,))
                .and_then(|call_result| call_result.extract::<Option<HitData>>());

            answered(py, inner, hit).flatten().map(|hit| hit.0)
        })
    }

    fn bounding_box(&self) -> Option<raydeon::AABB3<WorldSpace>> {
        Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let bounds = inner
                .call_method0("bounding_box")
                .and_then(|call_result| call_result.extract::<Option<AABB3>>());

            answered(py, inner, bounds)
                .flatten()
                .map(|aabb| aabb.0.cast_unit())
        })
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AxisAlignedCuboid>()?;
    m.add_class::<Tri>()?;
    m.add_class::<Plane>()?;
    m.add_class::<Quad>()?;
    m.add_class::<Sphere>()?;
    m.add_class::<Geometry>()?;
    m.add_class::<CollisionGeometry>()?;
    Ok(())
}
