pub mod address;
pub use address::*;
pub mod filter;
pub use filter::*;
use rendiation_math::Vec2;

pub use image::*;

pub trait Texture2DContainer {
  type Pixel;
  fn get(&self, position: Vec2<usize>) -> &Self::Pixel;
  fn get_mut(&mut self, position: Vec2<usize>) -> &mut Self::Pixel;
}

// impl Texture2DContainer for image::ImageBuffer {}
