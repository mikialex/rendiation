use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use rendiation_render_entity::color::{Color, LinearRGBColorSpace, RGBColor};

mod cook_torrance;

pub struct ScatteringEvent{
  pub out_dir: Vec3,
  pub brdf: Vec3,
  pub pdf: f32
}

pub trait Material{
  fn scatter(&self, in_dir: &Vec3, intersection: &Intersection) -> Option<ScatteringEvent>;
  fn sample_lighting(&self, intersection: &Intersection) -> Vec3;
}

#[derive(Clone, Copy)]
pub struct Lambertian {
    albedo: Color<LinearRGBColorSpace<f32>>,
}

impl Material for Lambertian{
  fn scatter(&self, _in_dir: &Vec3, intersection: &Intersection) -> Option<ScatteringEvent>{
    let (out_dir, cos) = cosine_sample_hemisphere_in_dir(intersection.hit_normal);
    let pdf = cos / PI;
    let brdf = self.albedo.value / Vec3::new(PI, PI, PI);
    Some(ScatteringEvent{
      out_dir,
      brdf,
      pdf
    })
    // // let (out_dir, cos) = cosine_sample_hemisphere_in_dir(intersection.hit_normal);
    // let pdf = in_dir.reflect(intersection.hit_normal).dot(intersection.hit_normal).abs();
    // let brdf = self.albedo.value;
    // Some(ScatteringEvent{
    //   out_dir: in_dir.reflect(intersection.hit_normal),
    //   brdf,
    //   pdf
    // })
  }

  fn sample_lighting(&self, _: &Intersection) -> Vec3{
    Vec3::new(0., 0., 0.)
  }
}

impl Lambertian {
  pub fn new() -> Self {
    Self {
      albedo: Color::from_value((0.95, 0.95, 0.95)),
    }
  }

  pub fn albedo(&mut self, r: f32, g: f32, b: f32) -> &Self {
    *self.albedo.mut_r() = r;
    *self.albedo.mut_g() = g;
    *self.albedo.mut_b() = b;
    self
  }
}
