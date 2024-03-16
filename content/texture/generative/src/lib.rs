#![allow(unused)]
use rendiation_algebra::*;

pub mod perlin;
pub mod worley;

pub trait TextureGenerator {
  type Pixel;
  fn gen(&self, p: Vec2<usize>) -> Self::Pixel;
}
