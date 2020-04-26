use rendiation_math::Vec3;

pub mod worley;
pub mod perlin;

pub trait NoiseFn3D{
  fn get(input: Vec3<f32>) -> f32;
}