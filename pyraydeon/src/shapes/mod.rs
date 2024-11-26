use primitive::{Plane, Quad, Sphere};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use raydeon::WorldSpace;
use std::sync::Arc;

mod primitive;

pub(crate) use primitive::{AxisAlignedCuboid, Tri};

use crate::material::Material;
use crate::ray::{HitData, Ray, AABB3};
use crate::scene::{Camera, LineSegment3D};

type RMaterial = raydeon::material::Material;

#[derive(Debug)]
enum InnerGeometry {
    Native(Arc<dyn raydeon::Shape<WorldSpace, RMaterial>>),
    Py,
}

#[derive(Debug)]
#[pyclass(subclass, frozen)]
pub(crate) struct Geometry {
    geom: InnerGeometry,
}

impl Geometry {
    pub(crate) fn native(geom: Arc<dyn raydeon::Shape<WorldSpace, RMaterial>>) -> Self {
        let geom = InnerGeometry::Native(geom);
        Self { geom }
    }

    pub(crate) fn py() -> Self {
        let geom = InnerGeometry::Py;
        Self { geom }
    }

    pub(crate) fn geometry(&self, obj: PyObject) -> Arc<dyn raydeon::Shape<WorldSpace, RMaterial>> {
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
                paths
                    .into_iter()
                    .map(raydeon::path::LineSegment3D::cast_unit)
                    .map(Into::into)
                    .collect()
            }),
            InnerGeometry::Py => Vec::new(),
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().geom))
    }
}

#[derive(Debug)]
enum InnerCollisionGeometry {
    Native(Arc<dyn raydeon::CollisionGeometry<WorldSpace>>),
    Py,
}

#[derive(Debug)]
#[pyclass(subclass, frozen)]
pub(crate) struct CollisionGeometry {
    geom: InnerCollisionGeometry,
}

impl CollisionGeometry {
    pub(crate) fn native(geom: Arc<dyn raydeon::CollisionGeometry<WorldSpace>>) -> Self {
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

impl raydeon::Shape<WorldSpace, raydeon::material::Material> for PythonGeometry {
    fn collision_geometry(&self) -> Option<Vec<Arc<dyn raydeon::CollisionGeometry<WorldSpace>>>> {
        let collision_geometry: Option<_> = Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let call_result = inner.call_method1("collision_geometry", ()).ok()?;

            let nullable: Option<Bound<'_, PyAny>> = call_result.extract().unwrap();
            let collision_iter = nullable?.iter().unwrap();

            let geometry: Vec<_> = collision_iter
                .map(|obj| {
                    Ok(
                        Arc::new(PythonGeometry::as_collision_geometry(obj?.into_py(py)))
                            as Arc<dyn raydeon::CollisionGeometry<WorldSpace>>,
                    )
                })
                .collect::<PyResult<_>>()
                .unwrap();

            Some(geometry)
        });
        collision_geometry
    }

    fn paths(
        &self,
        cam: &raydeon::Camera<raydeon::Perspective, raydeon::Observation>,
    ) -> Vec<raydeon::path::LineSegment3D<WorldSpace, raydeon::material::Material>> {
        let segments: Option<_> = Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let cam = Camera::from(cam.clone());
            let call_result = inner.call_method1("paths", (cam,)).unwrap();

            let segments = call_result
                .extract::<Option<Vec<LineSegment3D>>>()
                .unwrap()?;

            Some(
                segments
                    .into_iter()
                    .map(|segment| segment.0.cast_unit())
                    .collect(),
            )
        });
        segments.unwrap_or_default()
    }

    fn metadata(&self) -> raydeon::material::Material {
        let material: Option<Material> = Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let attr_value = inner.getattr("material").ok()?;

            let material: Material = attr_value.extract().ok()?;

            Some(material)
        });
        material.unwrap_or_default().0
    }
}

impl raydeon::CollisionGeometry<WorldSpace> for PythonGeometry {
    fn hit_by(&self, ray: &raydeon::Ray) -> Option<raydeon::HitData> {
        if let PythonGeometryKind::Collision { aabb: Some(aabb) } = &self.kind {
            raydeon::shapes::AxisAlignedCuboid::from(aabb.0.cast_unit()).hit_by(ray)?;
        }
        Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let ray = Ray::from(*ray);
            let call_result = inner.call_method1("hit_by", (ray,)).unwrap();

            call_result
                .extract::<Option<HitData>>()
                .ok()?
                .map(|hit| hit.0)
        })
    }

    fn bounding_box(&self) -> Option<raydeon::AABB3<WorldSpace>> {
        Python::with_gil(|py| {
            let inner = self.slf.bind(py);
            let call_result = inner.call_method1("bounding_box", ()).unwrap();

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
    m.add_class::<Plane>()?;
    m.add_class::<Quad>()?;
    m.add_class::<Sphere>()?;
    m.add_class::<Geometry>()?;
    m.add_class::<CollisionGeometry>()?;
    Ok(())
}
