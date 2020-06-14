use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use rendiation_render_entity::color::{Color, LinearRGBColorSpace, RGBColor};

pub struct ScatteringEvent{
  
}

pub trait Material{

}

#[derive(Clone, Copy)]
pub struct Lambertian {
    albedo: Color<LinearRGBColorSpace<f32>>,
}

impl Material for Lambertian{}

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
