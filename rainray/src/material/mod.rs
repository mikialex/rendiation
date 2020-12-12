use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use rendiation_render_entity::color::{Color, LinearRGBColorSpace, RGBColor};

mod physical;

pub struct ScatteringEvent {
  pub out_dir: Vec3,
  pub pdf: f32,
}

impl ScatteringEvent {
  pub fn create_next_ray(&self, at_position: Vec3) -> Ray3 {
    Ray3::new(at_position, self.out_dir)
  }
}

pub trait Material: Send + Sync {
  fn scatter(&self, in_dir: &Vec3, intersection: &Intersection) -> Option<ScatteringEvent>;
  fn bsdf(&self, in_dir: &Vec3, out_dir: &Vec3, intersection: &Intersection) -> Vec3;
  fn sample_emissive(&self, intersection: &Intersection) -> Vec3;
}

#[derive(Clone, Copy)]
pub struct Lambertian {
  albedo: Color<LinearRGBColorSpace<f32>>,
}

impl Material for Lambertian {
  fn scatter(&self, _in_dir: &Vec3, intersection: &Intersection) -> Option<ScatteringEvent> {
    let (out_dir, cos) = cosine_sample_hemisphere_in_dir(intersection.hit_normal);
    let pdf = cos / PI;
    Some(ScatteringEvent { out_dir, pdf })
  }

  fn bsdf(&self, in_dir: &Vec3, out_dir: &Vec3, intersection: &Intersection) -> Vec3 {
    self.albedo.value / Vec3::splat(PI)
  }

  fn sample_emissive(&self, _: &Intersection) -> Vec3 {
    Vec3::new(0., 0., 0.)
  }
}

impl Default for Lambertian {
  fn default() -> Self {
    Self {
      albedo: Color::from_value((0.95, 0.95, 0.95)),
    }
  }
}

impl Lambertian {
  pub fn albedo(&mut self, r: f32, g: f32, b: f32) -> &Self {
    *self.albedo.mut_r() = r;
    *self.albedo.mut_g() = g;
    *self.albedo.mut_b() = b;
    self
  }
}
