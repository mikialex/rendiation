#![allow(clippy::float_cmp)]
#![feature(nonzero_is_power_of_two)]

pub mod address;
use std::{
  num::NonZeroUsize,
  ops::{Deref, DerefMut},
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
pub mod util;
pub use util::*;
pub mod io;
pub use io::*;

use image::ImageBuffer;
use rendiation_algebra::{Lerp, Scalar, Vec2};

pub use image::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
  pub width: NonZeroUsize,
  pub height: NonZeroUsize,
}

impl Size {
  pub fn is_pot(&self) -> bool {
    self.width.is_power_of_two() && self.height.is_power_of_two()
  }

  pub fn from_u32_pair_min_one(size: (u32, u32)) -> Self {
    let width = NonZeroUsize::new(size.0 as usize).unwrap_or(NonZeroUsize::new(1).unwrap());
    let height = NonZeroUsize::new(size.1 as usize).unwrap_or(NonZeroUsize::new(1).unwrap());
    Size { width, height }
  }
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

  fn size(&self) -> Size;
  fn width(&self) -> usize {
    self.size().width.into()
  }
  fn height(&self) -> usize {
    self.size().width.into()
  }

  fn pixel_count(&self) -> usize {
    self.width() * self.height()
  }

  fn iter(&self) -> TexturePixels<'_, Self> {
    TexturePixels {
      texture: self,
      current: 0,
      all: self.pixel_count(),
    }
  }
}

/// Not all texture storage container has continues memory,
/// use this trait to get under laying buffer for GPU resource uploading
pub trait BufferLikeTexture2D: Texture2D {
  fn as_byte(&self) -> &[u8];
}

pub trait Texture2dSampleAble: Texture2D {
  #[inline]
  fn sample_impl<T, Address, Filter>(
    &self,
    position: Vec2<T>,
    address: Address,
    filter: Filter,
  ) -> Self::Pixel
  where
    T: Scalar + From<usize> + Into<usize>,
    Address: Fn(T) -> T,
    Filter: Fn(T, Self::Pixel, Self::Pixel) -> Self::Pixel,
  {
    let corrected = position.map(|v| address(v));
    let size = Vec2::new(self.width().into(), self.height().into());
    let sample_position = corrected.zip(size, |c, size| c * size);
    let min_x_min_y = sample_position.map(|v| v.floor().into());
    let max_x_max_y = sample_position.map(|v| v.ceil().into());
    let min_x_max_y = Vec2::new(min_x_min_y.x, max_x_max_y.y);
    let max_x_min_y = Vec2::new(max_x_max_y.x, min_x_min_y.y);
    let interpolate = sample_position.map(|v| v - v.floor());

    let min_y = filter(
      interpolate.x,
      self.read(min_x_min_y),
      self.read(max_x_min_y),
    );
    let max_y = filter(
      interpolate.x,
      self.read(min_x_max_y),
      self.read(max_x_max_y),
    );
    filter(interpolate.y, min_y, max_y)
  }

  fn sample<T, U, V>(&self, position: Vec2<T>) -> Self::Pixel
  where
    T: Scalar + From<usize> + Into<usize>,
    U: TextureAddressMode,
    V: TextureFilterMode<T, Self::Pixel>,
  {
    self.sample_impl(position, U::correct, V::interpolate)
  }

  fn sample_dyn<T>(
    &self,
    position: Vec2<T>,
    address: AddressMode,
    filter: FilterMode,
  ) -> Self::Pixel
  where
    T: Scalar + From<usize> + Into<usize>,
    Self::Pixel: Lerp<T>,
  {
    self.sample_impl(
      position,
      |v| address.correct(v),
      |v, a, b| filter.interpolate(v, a, b),
    )
  }
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

  fn size(&self) -> Size {
    let d = self.dimensions();
    Size {
      width: NonZeroUsize::new(d.0 as usize).unwrap(),
      height: NonZeroUsize::new(d.1 as usize).unwrap(),
    }
  }
}
