use crate::EPSILON;
use bon::{builder, Builder};
use float_cmp::{approx_eq, ApproxEq};

#[derive(Debug, Clone, Copy, Default, Builder)]
#[builder(start_fn(name = new))]
pub struct Material {
    #[builder(default)]
    pub diffuse: f64,
    #[builder(default)]
    pub specular: f64,
    #[builder(default)]
    pub shininess: f64,
    #[builder(default)]
    pub tag: usize,
}

impl Material {
    pub fn new_mat(diffuse: f64, specular: f64, shininess: f64, tag: usize) -> Self {
        Self {
            diffuse,
            specular,
            shininess,
            tag,
        }
    }
}

impl PartialEq for Material {
    fn eq(&self, other: &Self) -> bool {
        approx_eq!(&Material, self, other, epsilon = EPSILON)
    }
}

impl ApproxEq for &Material {
    type Margin = float_cmp::F64Margin;

    fn approx_eq<M: Into<Self::Margin>>(self, other: Self, margin: M) -> bool {
        let margin = margin.into();
        approx_eq!(f64, self.diffuse, other.diffuse, epsilon = margin.epsilon)
            && approx_eq!(f64, self.specular, other.specular, epsilon = margin.epsilon)
            && approx_eq!(
                f64,
                self.shininess,
                other.shininess,
                epsilon = margin.epsilon
            )
            && self.tag == other.tag
    }
}
