#[derive(Debug, Clone, Copy, Default)]
pub struct Material {
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
    pub tag: usize,
}

impl Material {
    pub fn new(diffuse: f64, specular: f64, shininess: f64, tag: usize) -> Self {
        Self {
            diffuse,
            specular,
            shininess,
            tag,
        }
    }
}
