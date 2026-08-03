use crate::hatch::HatchStyle;
use crate::stroke::PenId;
use bon::Builder;

/// How a surface takes light, and how it draws itself.
#[derive(Debug, Clone, PartialEq, Default, Builder)]
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
    /// How this material shades itself, if it does. Without a style the
    /// material draws outlines only.
    pub hatch: Option<HatchStyle>,
}
