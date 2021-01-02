use crate::light::*;
use crate::model::*;
use crate::ray::*;
use crate::{environment::*, Vec3};
use rendiation_math::*;
use rendiation_math_entity::Ray3;
use std::sync::Arc;

pub struct Scene {
  pub models: Vec<Model>,
  pub point_lights: Vec<PointLight>,
  pub lights: Vec<Box<dyn Light>>,
  pub env: Box<dyn Environment>,
}

impl AsMut<Self> for Scene {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl Default for Scene {
  fn default() -> Self {
    Self {
      models: Vec::new(),
      point_lights: Vec::new(),
      lights: Vec::new(),
      env: Box::new(SolidEnvironment::black()),
    }
  }
}

// copy from RTX gem
const ORIGIN: f32 = 1.0 / 32.0;
const FLOAT_SCALE: f32 = 1.0 / 65536.0;
const INT_SCALE: f32 = 256.0;

#[inline(always)]
fn float_as_int(f: f32) -> i32 {
  unsafe { std::mem::transmute(f) }
}
#[inline(always)]
fn int_as_float(f: i32) -> f32 {
  unsafe { std::mem::transmute(f) }
}

// Normal points outward for rays exiting the surface, else is flipped.
#[rustfmt::skip]
fn offset_ray(p: Vec3, n: Vec3) -> Vec3 {
  let of_i = n.map(|n| (n * INT_SCALE) as i32);
  let p_i = p.zip(of_i, |p, of_i_p| {
    int_as_float(float_as_int(p) + (if p < 0. { -of_i_p } else { of_i_p }))
  });

   Vec3::new(
     if p.x.abs() < ORIGIN { p.x + FLOAT_SCALE * n.x } else { p_i.x },
     if p.y.abs() < ORIGIN { p.y + FLOAT_SCALE * n.y } else { p_i.y },
     if p.z.abs() < ORIGIN { p.z + FLOAT_SCALE * n.z } else { p_i.z },
   )
}

impl Scene {
  pub fn get_min_dist_hit(&self, ray: Ray3) -> Option<(Intersection, &Model)> {
    let mut min_distance = std::f32::INFINITY;
    let mut result: Option<(Intersection, &Model)> = None;
    for model in &self.models {
      if let PossibleIntersection(Some(mut intersection)) = model.geometry.intersect(&ray, &()) {
        if intersection.distance < min_distance {
          // intersection.hit_position = intersection.hit_position + intersection.hit_normal * 0.001;
          intersection.hit_position =
            offset_ray(intersection.hit_position, intersection.hit_normal.value);
          min_distance = intersection.distance;
          result = Some((intersection, model))
        }
      }
    }
    result
  }

  pub fn test_point_visible_to_point(&self, point_a: Vec3, point_b: Vec3) -> bool {
    let ray = Ray3::from_point_to_point(point_a, point_b);
    let distance = (point_a - point_b).length();

    if let Some(hit_result) = self.get_min_dist_hit(ray) {
      hit_result.0.distance > distance
    } else {
      true
    }
  }

  pub fn environment(&mut self, env: impl Environment) -> &mut Self {
    self.env = Box::new(env);
    self
  }

  pub fn model(&mut self, model: Model) -> &mut Self {
    self.models.push(model);
    self
  }

  pub fn light(&mut self, light: impl Light) -> &mut Self {
    self.lights.push(Box::new(light));
    self
  }
}
