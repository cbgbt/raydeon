use crate::stroke::PenId;
use bon::Builder;

#[derive(Debug, Clone, Copy, PartialEq, Default, Builder)]
#[builder(start_fn(name = new))]
#[non_exhaustive]
pub struct Material {
    #[builder(default)]
    pub diffuse: f64,
    #[builder(default)]
    pub specular: f64,
    #[builder(default)]
    pub shininess: f64,
    /// The pen which draws this material's strokes.
    #[builder(default)]
    pub pen: PenId,
}
