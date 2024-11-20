use numpy::{Ix1, PyArray, PyReadonlyArray1};
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use raydeon::CollisionGeometry as _;

use crate::ray::{HitData, Ray};

#[derive(Debug, Copy, Clone)]
pub struct ArbitrarySpace;

pywrap!(Vec3, raydeon::Vec3<ArbitrarySpace>);

#[pymethods]
impl Vec3 {
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        raydeon::Vec3::new(x, y, z).into()
    }

    fn __len__(&self) -> usize {
        3
    }

    fn as_point(slf: PyRef<'_, Self>) -> PyResult<Py<Point3>> {
        Py::new(slf.py(), Point3(slf.0.to_point()))
    }

    fn __getitem__(&self, idx: usize) -> PyResult<f64> {
        match idx {
            0 => Ok(self.0.x),
            1 => Ok(self.0.y),
            2 => Ok(self.0.z),
            _ => Err(PyIndexError::new_err(format!("'{idx}' is out of bounds"))),
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.0.x
    }

    #[getter]
    fn y(&self) -> f64 {
        self.0.y
    }

    #[getter]
    fn z(&self) -> f64 {
        self.0.z
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<FloatIter>> {
        let iter = FloatIter {
            iter: Box::new([slf.0.x, slf.0.y, slf.0.z].into_iter()),
        };
        Py::new(slf.py(), iter)
    }

    fn __add__(slf: &Bound<'_, Self>, other: &Bound<'_, Self>) -> Self {
        let slf = slf.borrow();
        let other = other.borrow();
        (slf.0 + other.0).into()
    }

    fn __sub__(slf: &Bound<'_, Self>, other: &Bound<'_, Self>) -> Self {
        let slf = slf.borrow();
        let other = other.borrow();
        (slf.0 - other.0).into()
    }

    fn __mul__(slf: &Bound<'_, Self>, other: f64) -> Self {
        let slf = slf.borrow();
        (slf.0 * other).into()
    }
}

impl TryFrom<&Bound<'_, PyAny>> for Vec3 {
    type Error = PyErr;

    fn try_from(value: &Bound<'_, PyAny>) -> Result<Self, Self::Error> {
        let x = value.get_item(0)?.extract()?;
        let y = value.get_item(1)?.extract()?;
        let z = value.get_item(2)?.extract()?;
        Ok(Self::new(x, y, z))
    }
}

impl TryFrom<PyReadonlyArray1<'_, f64>> for Vec3 {
    type Error = PyErr;

    fn try_from(value: PyReadonlyArray1<f64>) -> Result<Self, Self::Error> {
        let value = value
            .as_slice()
            .map_err(|_| pyo3::exceptions::PyValueError::new_err("hit_point must be a 1D array"))?;
        Ok(Self::new(value[0], value[1], value[2]))
    }
}

pywrap!(Point3, raydeon::Point3<ArbitrarySpace>);

#[pymethods]
impl Point3 {
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        raydeon::Point3::new(x, y, z).into()
    }

    fn __len__(&self) -> usize {
        3
    }

    fn __getitem__(&self, idx: usize) -> PyResult<f64> {
        match idx {
            0 => Ok(self.0.x),
            1 => Ok(self.0.y),
            2 => Ok(self.0.z),
            _ => Err(PyIndexError::new_err(format!("'{idx}' is out of bounds"))),
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.0.x
    }

    #[getter]
    fn y(&self) -> f64 {
        self.0.y
    }

    #[getter]
    fn z(&self) -> f64 {
        self.0.z
    }

    fn as_vec(slf: PyRef<'_, Self>) -> PyResult<Py<Vec3>> {
        Py::new(slf.py(), Vec3(slf.0.to_vector()))
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<FloatIter>> {
        let iter = FloatIter {
            iter: Box::new([slf.0.x, slf.0.y, slf.0.z].into_iter()),
        };
        Py::new(slf.py(), iter)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

impl TryFrom<&Bound<'_, PyAny>> for Point3 {
    type Error = PyErr;

    fn try_from(value: &Bound<'_, PyAny>) -> Result<Self, Self::Error> {
        let x = value.get_item(0)?.extract()?;
        let y = value.get_item(1)?.extract()?;
        let z = value.get_item(2)?.extract()?;
        Ok(Self::new(x, y, z))
    }
}

impl TryFrom<PyReadonlyArray1<'_, f64>> for Point3 {
    type Error = PyErr;

    fn try_from(value: PyReadonlyArray1<f64>) -> Result<Self, Self::Error> {
        let value = value
            .as_slice()
            .map_err(|_| pyo3::exceptions::PyValueError::new_err("hit_point must be a 1D array"))?;
        Ok(Self::new(value[0], value[1], value[2]))
    }
}

pywrap!(Point2, raydeon::Point2<ArbitrarySpace>);

#[pymethods]
impl Point2 {
    #[new]
    fn new(x: f64, y: f64) -> Self {
        raydeon::Point2::new(x, y).into()
    }

    #[getter]
    fn x(&self) -> f64 {
        self.0.x
    }

    #[getter]
    fn y(&self) -> f64 {
        self.0.y
    }

    fn __len__(&self) -> usize {
        2
    }

    fn __getitem__(&self, idx: usize) -> PyResult<f64> {
        match idx {
            0 => Ok(self.0.x),
            1 => Ok(self.0.y),
            _ => Err(PyIndexError::new_err(format!("'{idx}' is out of bounds"))),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<FloatIter>> {
        let iter = FloatIter {
            iter: Box::new([slf.0.x, slf.0.y].into_iter()),
        };
        Py::new(slf.py(), iter)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

impl TryFrom<PyReadonlyArray1<'_, f64>> for Point2 {
    type Error = PyErr;

    fn try_from(value: PyReadonlyArray1<f64>) -> Result<Self, Self::Error> {
        let value = value
            .as_slice()
            .map_err(|_| pyo3::exceptions::PyValueError::new_err("hit_point must be a 1D array"))?;
        Ok(Self::new(value[0], value[1]))
    }
}

#[pyclass]
struct FloatIter {
    iter: Box<dyn Iterator<Item = f64> + Send + Sync>,
}

#[pymethods]
impl FloatIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<f64> {
        slf.iter.next()
    }
}

pywrap!(AABB3, raydeon::AABB3<ArbitrarySpace>);

#[pymethods]
impl AABB3 {
    #[new]
    fn new(min: PyReadonlyArray1<f64>, max: PyReadonlyArray1<f64>) -> PyResult<Self> {
        let min = Point3::try_from(min)?;
        let max = Point3::try_from(max)?;
        Ok(raydeon::AABB3::new(min.0, max.0).into())
    }

    #[getter]
    fn min<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.min.to_array())
    }

    #[getter]
    fn max<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.max.to_array())
    }

    fn hit_by(&self, _py: Python, ray: Ray) -> Option<HitData> {
        raydeon::shapes::AxisAlignedCuboid::from(self.0.cast_unit())
            .hit_by(&ray.0)
            .map(Into::into)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().0))
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Vec3>()?;
    m.add_class::<Point3>()?;
    m.add_class::<AABB3>()?;
    Ok(())
}
