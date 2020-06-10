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

  pub fn next_ray(&self, into_ray: &Ray3, intersection: &Intersection) -> Ray3 {
    // Ray3::new(
    //     intersection.hit_position,
    //     cosine_sample_hemisphere(&intersection.hit_normal),
    // )
    // Ray3::new(
    //     intersection.hit_position,
    //     Vec3::reflect(&intersection.hit_normal, &into_ray.direction),
    // )

    Ray3::from_point_to_point(
      intersection.hit_position,
      intersection.hit_position + intersection.hit_normal + rand_point_in_unit_sphere(),
    )

    // Ray3::from_point_to_point(
    //     &intersection.hit_position,
    //     &(intersection.hit_position
    //         + intersection.hit_normal
    //         + 0.5 * rand_point_in_unit_sphere()
    //         + Vec3::reflect(&intersection.hit_normal, &into_ray.direction)),
    // )
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
