pub mod address;
use std::{
  ops::{Deref, DerefMut},
  path::Path,
};

pub use address::*;
pub mod filter;
pub use filter::*;
pub mod cube;
pub use cube::*;
pub mod sampler;
pub use sampler::*;
pub mod iter;
pub use iter::*;

use image::ImageBuffer;
use rendiation_algebra::Vec2;

pub use image::*;

pub struct Size<T> {
  width: T,
  height: T,
}

pub trait Texture2D: Sized {
  type Pixel: Copy;
  fn get(&self, position: Vec2<usize>) -> &Self::Pixel;
  fn get_mut(&mut self, position: Vec2<usize>) -> &mut Self::Pixel;

  fn read(&self, position: Vec2<usize>) -> Self::Pixel {
    *self.get(position)
  }
  fn write(&mut self, position: Vec2<usize>, v: Self::Pixel) {
    *self.get_mut(position) = v;
  }

  fn size(&self) -> Size<usize>;

  fn pixel_count(&self) -> usize {
    let Size { width, height } = self.size();
    width * height
  }

  fn iter<'a>(&'a self) -> TexturePixels<'a, Self> {
    TexturePixels {
      texture: self,
      current: 0,
      all: self.pixel_count(),
    }
  }

  fn save_to_file<P: AsRef<Path>>(&self, path: P);
}

impl<P, C> Texture2D for ImageBuffer<P, C>
where
  P: Pixel + 'static,
  [P::Subpixel]: EncodableLayout,
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

  fn size(&self) -> Size<usize> {
    let d = self.dimensions();
    Size {
      width: d.0 as usize,
      height: d.1 as usize,
    }
  }

  fn save_to_file<Pa: AsRef<Path>>(&self, path: Pa) {
    self.save(path).unwrap();
  }
}
