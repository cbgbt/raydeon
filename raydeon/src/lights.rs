use ray::HitShape;

use crate::*;

pub trait Light: std::fmt::Debug + Send + Sync + 'static {
    fn compute_illumination<'s>(&self, scene: &'s Scene, hit_shape: HitShape<'s>) -> f64;
}

#[derive(Debug, Copy, Clone, Default)]
pub struct PointLight {
    intensity: f64,
    specular_intensity: f64,
    position: WPoint3,

    constant_attenuation: f64,
    linear_attenuation: f64,
    quadratic_attenuation: f64,
}

impl Light for PointLight {
    fn compute_illumination<'s>(&self, scene: &'s Scene, hit_shape: HitShape<'s>) -> f64 {
        let _light_hitpoint = match self.light_hitpoint_for_hit(scene, hit_shape) {
            Some(hit) => hit,
            None => return 0.0,
        };

        let mut illum = 0.0;

        illum += self.diffuse_illumination(hit_shape);
        let specular = self.specular_illumination(hit_shape);
        tracing::debug!("specular: {}", specular);
        illum += specular;

        let atten = self.attenuation(hit_shape);
        tracing::debug!("pre-attenuated illum: {}", illum);
        tracing::debug!("atten: {}", atten);
        let illum = illum * atten;

        tracing::debug!("illum: {}", illum);
        illum
    }
}

impl PointLight {
    pub fn new(
        intensity: f64,
        specular_intensity: f64,
        position: impl Into<WPoint3>,
        constant_attenuation: f64,
        linear_attenuation: f64,
        quadratic_attenuation: f64,
    ) -> Self {
        let position = position.into();
        Self {
            intensity,
            specular_intensity,
            position,
            constant_attenuation,
            linear_attenuation,
            quadratic_attenuation,
        }
    }

    pub fn intensity(&self) -> f64 {
        self.intensity
    }

    pub fn specular(&self) -> f64 {
        self.specular_intensity
    }

    pub fn position(&self) -> WPoint3 {
        self.position
    }

    pub fn constant_attenuation(&self) -> f64 {
        self.constant_attenuation
    }

    pub fn linear_attenuation(&self) -> f64 {
        self.linear_attenuation
    }

    pub fn quadratic_attenuation(&self) -> f64 {
        self.quadratic_attenuation
    }

    fn diffuse_illumination(&self, hit_shape: HitShape) -> f64 {
        let hitpoint = &hit_shape.hit_data;
        let material = hit_shape.hit_shape.material().unwrap_or_default();
        let to_light = (self.position - hitpoint.hit_point).normalize();
        let diffuse_scale = to_light.dot(hitpoint.normal).max(0.0);
        material.diffuse * self.intensity * diffuse_scale
    }

    fn specular_illumination(&self, hit_shape: HitShape) -> f64 {
        let hitpoint = hit_shape.hit_data;
        let material = hit_shape.hit_shape.material().unwrap_or_default();
        let to_light = (self.position - hitpoint.hit_point).normalize();

        let v = hitpoint.hit_point.to_vector() * -1.0;
        let h = (to_light + v).normalize();

        let ps = material.specular * self.specular_intensity;
        let blinn_phong = h.dot(hitpoint.normal).max(0.0).powf(material.shininess);
        let blinn_phong = blinn_phong / (8.0 * std::f64::consts::PI / (material.shininess + 2.0));

        ps * blinn_phong
    }

    fn attenuation(&self, hitpoint: HitShape) -> f64 {
        let distance = (self.position - hitpoint.hit_data.hit_point).length();
        let attenuation = self.constant_attenuation
            + self.linear_attenuation * distance
            + self.quadratic_attenuation * distance * distance;
        1.0 / attenuation
    }

    fn light_hitpoint_for_hit<'s>(
        &self,
        scene: &'s Scene,
        hit_shape: HitShape<'s>,
    ) -> Option<HitShape<'s>> {
        let hitpoint = hit_shape.hit_data;
        let to_light = (self.position - hitpoint.hit_point).normalize();
        if to_light.dot(hitpoint.normal) < 0.0 {
            return None;
        }

        let to_hitpoint = (hitpoint.hit_point - self.position).normalize();
        let light_ray = Ray::new(self.position, to_hitpoint);
        scene.intersects(light_ray).and_then(|light_hitpoint| {
            Arc::ptr_eq(
                &light_hitpoint.hit_shape.geometry,
                &hit_shape.hit_shape.geometry,
            )
            .then_some(light_hitpoint)
        })
    }
}
