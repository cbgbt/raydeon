use pyo3::prelude::*;

macro_rules! pywrap {
    ($name:ident, $wraps:ty) => {
        #[derive(Debug, Clone)]
        #[pyclass(frozen)]
        pub(crate) struct $name(pub(crate) $wraps);

        impl ::std::ops::Deref for $name {
            type Target = $wraps;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<$wraps> for $name {
            fn from(value: $wraps) -> Self {
                Self(value)
            }
        }
    };
}

mod camera;
mod drawables;
mod hatch;
mod light;
mod linear;
mod material;
mod ray;
mod scene;
mod shapes;

/// A Python module implemented in Rust.
#[pymodule]
fn pyraydeon(m: &Bound<'_, PyModule>) -> PyResult<()> {
    crate::linear::register(m)?;

    crate::camera::register(m)?;
    crate::scene::register(m)?;

    crate::ray::register(m)?;

    crate::shapes::register(m)?;
    crate::hatch::register(m)?;
    crate::material::register(m)?;
    crate::light::register(m)?;

    crate::drawables::register(m)?;

    Ok(())
}
