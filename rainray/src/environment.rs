use crate::math::vec3::Vec3;
use crate::ray::*;

pub struct SolidEnvironment {
    pub intensity: Vec3,
}

pub trait Environment {
    fn sample(&self, ray: &Ray) -> Vec3;
}

impl Environment for SolidEnvironment {
    fn sample(&self, _ray: &Ray) -> Vec3 {
        self.intensity
    }
}

pub struct GradientEnvironment {
    pub top_intensity: Vec3,
    pub bottom_intensity: Vec3,
}

impl Environment for GradientEnvironment {
    fn sample(&self, ray: &Ray) -> Vec3 {
        let t = ray.direction.y / 2.0 + 1.;
        Vec3::new(
            lerp(t, self.bottom_intensity.x, self.top_intensity.x),
            lerp(t, self.bottom_intensity.y, self.top_intensity.y),
            lerp(t, self.bottom_intensity.z, self.top_intensity.z),
        )
    }
}

fn lerp(t: f64, min: f64, max: f64) -> f64 {
    (1. - t) * max + t * min
}
