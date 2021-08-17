#![allow(unused)]
#![allow(unstable_name_collisions)]
use rendiation_algebra::Vec3;

pub mod perlin;
pub mod worley;

pub trait NoiseFn3D {
  fn get(input: Vec3<f32>) -> f32;
}
