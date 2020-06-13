use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use rendiation_render_entity::color::{Color, LinearRGBColorSpace, RGBColor};

#[derive(Clone, Copy)]
pub struct Material {
  pub albedo: Color<LinearRGBColorSpace<f32>>,
  pub emissive: Vec3,
}

impl Material {
  pub fn new() -> Material {
    Material {
      albedo: Color::from_value((0.95, 0.95, 0.95)),
      emissive: Vec3::new(0.0, 0.0, 0.0),
    }
  }

  pub fn color(&mut self, r: f32, g: f32, b: f32) -> &Self {
    self.albedo.mut_r(r);
    self.albedo.mut_g(g);
    self.albedo.mut_b(b);
    self
  }

  pub fn collect_energy(&self, look_up_ray: &Ray3) -> Vec3 {
    self.emissive
  }

  pub fn brdf_importance_pdf(
    &self,
    intersection: &Intersection,
    in_ray: &Ray3,
    out_ray: &Ray3,
  ) -> f32 {
    1.
  }

  pub fn brdf(&self, intersection: &Intersection, in_ray: &Ray3, out_ray: &Ray3) -> f32 {
    let w_m = (-in_ray.direction + out_ray.direction).normalize();
    0.8
  }
}
