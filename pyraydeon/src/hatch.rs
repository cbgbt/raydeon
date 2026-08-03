//! The Python view of world-space hatching.
//!
//! A material shades itself with a [`HatchStyle`], and the surfaces the style
//! draws on are handed over by the shape. Python describes those surfaces in
//! plain arrays; they are parsed here, once, so that everything downstream
//! works with a surface which is known to make sense.

use numpy::{Ix1, PyArray, PyArrayLike2};
use pyo3::exceptions::{PyIndexError, PyTypeError, PyValueError};
use pyo3::prelude::*;

use crate::linear::Point3;

pywrap!(HatchStyle, raydeon::HatchStyle);

#[pymethods]
impl HatchStyle {
    /// Four-direction engraving, tightening as the surface darkens.
    #[staticmethod]
    #[pyo3(signature = (*, spacing))]
    fn tonal_crosshatch(spacing: f64) -> PyResult<Self> {
        Ok(raydeon::HatchStyle::tonal_crosshatch(hatch_spacing(spacing)?).into())
    }

    /// A single diagonal direction which thins out towards the light.
    #[staticmethod]
    #[pyo3(signature = (*, spacing))]
    fn stochastic(spacing: f64) -> PyResult<Self> {
        Ok(raydeon::HatchStyle::stochastic(hatch_spacing(spacing)?).into())
    }

    /// Strokes which follow the light across each surface, crossed only in
    /// deep shadow. `source` is where the light comes from.
    #[staticmethod]
    #[pyo3(signature = (*, source, spacing))]
    fn light_flow(source: &Bound<'_, PyAny>, spacing: f64) -> PyResult<Self> {
        let source = Point3::try_from(source)?;
        Ok(raydeon::HatchStyle::light_flow(source.0.cast_unit(), hatch_spacing(spacing)?).into())
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().0))
    }
}

fn hatch_spacing(spacing: f64) -> PyResult<raydeon::HatchSpacing> {
    raydeon::HatchSpacing::try_new(spacing).map_err(|err| PyValueError::new_err(err.to_string()))
}

pywrap!(PlanarSurface, raydeon::PlanarSurface);

#[pymethods]
impl PlanarSurface {
    /// A convex planar patch of a shape's surface.
    ///
    /// `basis` is a 2x3 array of perpendicular in-plane vectors whose cross
    /// product points away from the shape. `outline` is an Nx2 array of
    /// corners, and `holes` an Nx4 array of `(min_x, min_y, max_x, max_y)`
    /// openings, both written in units of those basis vectors.
    #[new]
    #[pyo3(signature = (origin, basis, outline, holes=None))]
    fn new(
        origin: &Bound<'_, PyAny>,
        basis: PyArrayLike2<'_, f64>,
        outline: PyArrayLike2<'_, f64>,
        holes: Option<PyArrayLike2<'_, f64>>,
    ) -> PyResult<Self> {
        let origin: Point3 = origin.try_into()?;
        let basis = rows(&basis, 3, "basis must be a 2x3 array")?;
        let basis: [raydeon::WVec3; 2] = match basis.as_slice() {
            [first, second] => [
                raydeon::WVec3::new(first[0], first[1], first[2]),
                raydeon::WVec3::new(second[0], second[1], second[2]),
            ],
            _ => return Err(PyIndexError::new_err("basis must be a 2x3 array")),
        };

        let outline = rows(&outline, 2, "outline must be an Nx2 array")?
            .into_iter()
            .map(|corner| raydeon::FacePoint::new(corner[0], corner[1]))
            .collect();
        let holes = holes
            .map(|holes| rows(&holes, 4, "holes must be an Nx4 array"))
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(|hole| {
                raydeon::FaceBox::new(
                    raydeon::FacePoint::new(hole[0], hole[1]),
                    raydeon::FacePoint::new(hole[2], hole[3]),
                )
            })
            .collect();

        raydeon::PlanarSurface::try_new(origin.0.cast_unit(), basis, outline, holes)
            .map(Into::into)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    /// The outward normal: the side the hatching is seen from.
    #[getter]
    fn normal<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.normal().to_array())
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().0))
    }
}

/// Reads a 2D numpy array as rows of a fixed width.
fn rows(array: &PyArrayLike2<'_, f64>, width: usize, complaint: &str) -> PyResult<Vec<Vec<f64>>> {
    let array = array.as_array();
    let flat = array
        .as_slice()
        .ok_or_else(|| PyIndexError::new_err(complaint.to_owned()))?;
    if array.ncols() != width {
        return Err(PyIndexError::new_err(complaint.to_owned()));
    }
    Ok(flat.chunks(width).map(<[f64]>::to_vec).collect())
}

pywrap!(SphereSurface, raydeon::SphereSurface);

#[pymethods]
impl SphereSurface {
    /// A whole sphere, hatched with contour rings rather than straight lines.
    #[new]
    fn new(center: &Bound<'_, PyAny>, radius: f64) -> PyResult<Self> {
        let center: Point3 = center.try_into()?;
        raydeon::SphereSurface::try_new(center.0.cast_unit(), radius)
            .map(Into::into)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    #[getter]
    fn center<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.0.center().to_array())
    }

    #[getter]
    fn radius(&self) -> f64 {
        self.0.radius()
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:#?}>", class_name, slf.borrow().0))
    }
}

/// Reads a surface a Python shape offers up for hatching.
pub(crate) fn hatch_surface_from_py(obj: &Bound<'_, PyAny>) -> PyResult<raydeon::HatchSurface> {
    if let Ok(planar) = obj.extract::<PlanarSurface>() {
        return Ok(raydeon::HatchSurface::Planar(planar.0));
    }
    obj.extract::<SphereSurface>()
        .map(|sphere| raydeon::HatchSurface::Sphere(sphere.0))
        .map_err(|_| {
            PyTypeError::new_err("a hatch surface must be a PlanarSurface or a SphereSurface")
        })
}

/// Hands a surface a native shape offers back to Python.
pub(crate) fn hatch_surface_into_py(py: Python<'_>, surface: raydeon::HatchSurface) -> PyObject {
    match surface {
        raydeon::HatchSurface::Planar(planar) => PlanarSurface::from(planar).into_py(py),
        raydeon::HatchSurface::Sphere(sphere) => SphereSurface::from(sphere).into_py(py),
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HatchStyle>()?;
    m.add_class::<PlanarSurface>()?;
    m.add_class::<SphereSurface>()?;
    Ok(())
}
