pub mod address;
use std::ops::{Deref, DerefMut};

pub use address::*;
pub mod filter;
pub use filter::*;
pub mod cube;
pub use cube::*;

use image::ImageBuffer;
use rendiation_algebra::Vec2;

pub use image::*;

pub trait Texture2D {
  type Pixel;
  fn get(&self, position: Vec2<usize>) -> &Self::Pixel;
  fn get_mut(&mut self, position: Vec2<usize>) -> &mut Self::Pixel;
  fn size(&self) -> Vec2<usize>;
}

impl<P, C> Texture2D for ImageBuffer<P, C>
where
  P: Pixel + 'static,
  C: Deref<Target = [P::Subpixel]>,
  C: DerefMut<Target = [P::Subpixel]>,
{
  type Pixel = P;

  fn get(&self, position: Vec2<usize>) -> &Self::Pixel {
    self.get_pixel(position.x as u32, position.y as u32)
  }

  fn get_mut(&mut self, position: Vec2<usize>) -> &mut Self::Pixel {
    self.get_pixel_mut(position.x as u32, position.y as u32)
  }

  fn size(&self) -> Vec2<usize> {
    let d = self.dimensions();
    (d.0 as usize, d.1 as usize).into()
  }
}
