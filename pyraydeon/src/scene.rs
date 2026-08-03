use std::sync::Arc;

use numpy::{Ix1, PyArray};
use pyo3::prelude::*;
use raydeon::SceneLighting;

use crate::camera::Camera;
use crate::drawables::{DrawableShape, Stroke};
use crate::light::PointLight;
use crate::linear::Point3;

#[pyclass(frozen)]
pub(crate) struct Scene {
    scene: Arc<raydeon::Scene>,
}

#[pymethods]
impl Scene {
    #[new]
    #[pyo3(signature = (geometry, lights=None, ambient_light=0.0))]
    fn new(
        geometry: Option<Vec<DrawableShape>>,
        lights: Option<Vec<PointLight>>,
        ambient_light: f64,
    ) -> PyResult<Self> {
        let geometry = geometry.unwrap_or_default();
        let geometry: Vec<_> = geometry
            .iter()
            .map(DrawableShape::raydeon_drawable)
            .collect::<_>();

        let lights = lights.unwrap_or_default();
        let lights: Vec<Arc<dyn raydeon::Light>> = lights
            .into_iter()
            .map(|l| Arc::new(l.0) as Arc<dyn raydeon::lights::Light>)
            .collect();

        let lighting = SceneLighting::new()
            .with_lights(lights)
            .with_ambient_lighting(ambient_light);
        let scene = Arc::new(
            raydeon::Scene::new()
                .geometry(geometry)
                .lighting(lighting)
                .build(),
        );
        Ok(Self { scene })
    }

    /// Draws the scene: shape outlines plus the hatching materials ask for.
    ///
    /// `seed` salts the position-hashed jitter of the hatching, so the same
    /// scene and seed always draw the same strokes in the same order.
    #[pyo3(signature = (camera, *, seed=0))]
    fn render(&self, py: Python, camera: &Camera, seed: u64) -> Vec<Stroke> {
        py.allow_threads(|| {
            let cam = self.scene.attach_camera(camera.0.clone()).with_seed(seed);
            cam.render().strokes().iter().map(Into::into).collect()
        })
    }

    /// Draws the scene, then shades the whole image with screen-space
    /// hatching laid out on the page rather than on the surfaces.
    #[pyo3(signature = (camera, *, seed=0))]
    fn render_with_screen_hatching(&self, py: Python, camera: &Camera, seed: u64) -> Vec<Stroke> {
        py.allow_threads(|| {
            let cam = self.scene.attach_camera(camera.0.clone()).with_seed(seed);
            cam.render_with_screen_hatching()
                .strokes()
                .iter()
                .map(Into::into)
                .collect()
        })
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow().scene))
    }
}

#[derive(Debug, Copy, Clone)]
#[pyclass(frozen)]
pub(crate) struct LineSegment3D {
    p1: [f64; 3],
    p2: [f64; 3],
}

#[pymethods]
impl LineSegment3D {
    #[new]
    fn new(p1: &Bound<'_, PyAny>, p2: &Bound<'_, PyAny>) -> PyResult<Self> {
        let p1 = Point3::try_from(p1)?;
        let p2 = Point3::try_from(p2)?;
        Ok(Self {
            p1: p1.to_array(),
            p2: p2.to_array(),
        })
    }

    #[getter]
    fn p1<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.p1)
    }

    #[getter]
    fn p2<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray<f64, Ix1>> {
        PyArray::from_slice_bound(py, &self.p2)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let class_name = slf.get_type().qualname()?;
        Ok(format!("{}<{:?}>", class_name, slf.borrow()))
    }
}

impl<Space> From<raydeon::LineSegment3D<Space>> for LineSegment3D
where
    Space: Copy + Clone + std::fmt::Debug,
{
    fn from(value: raydeon::LineSegment3D<Space>) -> Self {
        Self {
            p1: value.p1().to_array(),
            p2: value.p2().to_array(),
        }
    }
}

impl<Space> From<LineSegment3D> for raydeon::LineSegment3D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    fn from(value: LineSegment3D) -> Self {
        raydeon::LineSegment3D::new_segment(value.p1, value.p2)
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Scene>()?;
    m.add_class::<LineSegment3D>()?;
    Ok(())
}
