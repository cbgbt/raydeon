use pyo3::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct ArbitrarySpace;

pywrap!(Vec3, raydeon::Vec3<ArbitrarySpace>);

#[pymethods]
impl Vec3 {
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        raydeon::Vec3::new(x, y, z).into()
    }

    fn as_point(slf: PyRef<'_, Self>) -> PyResult<Py<Point3>> {
        Py::new(slf.py(), Point3(slf.0.to_point()))
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

pywrap!(Point3, raydeon::Point3<ArbitrarySpace>);

#[pymethods]
impl Point3 {
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        raydeon::Point3::new(x, y, z).into()
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
    fn new(min: &Point3, max: &Point3) -> Self {
        raydeon::AABB3::new(min.0, max.0).into()
    }

    #[getter]
    fn min(&self) -> Point3 {
        self.0.min.into()
    }

    #[getter]
    fn max(&self) -> Point3 {
        self.0.max.into()
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
