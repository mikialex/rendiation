#![allow(clippy::float_cmp)]

mod address;

pub use address::*;
mod filter;
pub use filter::*;
mod cube;
pub use cube::*;
mod sampler;
pub use sampler::*;
mod iter;
pub use iter::*;
mod util;
pub use container::*;
pub use util::*;
mod container;
use rendiation_algebra::{Lerp, Scalar, Vec2};
pub use rendiation_texture_types::*;
pub use wgpu_types::TextureFormat;

pub trait Texture2D: Sized {
  type Pixel: Copy;

  fn get(&self, position: impl Into<Vec2<usize>>) -> &Self::Pixel;
  fn get_mut(&mut self, position: impl Into<Vec2<usize>>) -> &mut Self::Pixel;

  fn read(&self, position: impl Into<Vec2<usize>>) -> Self::Pixel {
    *self.get(position)
  }
  fn write(&mut self, position: impl Into<Vec2<usize>>, v: Self::Pixel) {
    *self.get_mut(position.into()) = v;
  }

  fn size(&self) -> Size;
  fn width(&self) -> usize {
    self.size().width.into()
  }
  fn height(&self) -> usize {
    self.size().height.into()
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

  fn iter_mut(&mut self) -> TexturePixelsMut<'_, Self> {
    let all = self.pixel_count();
    TexturePixelsMut {
      texture: self,
      current: 0,
      all,
    }
  }

  fn clear(&mut self, pixel: Self::Pixel) {
    self.iter_mut().for_each(|(p, _)| *p = pixel)
  }

  fn map<T: Texture2dInitAble>(&self, mapper: impl Fn(Self::Pixel) -> T::Pixel) -> T {
    let mut target = T::init_not_care(self.size());
    self.iter().for_each(|(&p, xy)| {
      let p = mapper(p);
      target.write(xy, p)
    });
    target
  }

  fn fill_by(&mut self, writer: impl Fn(Vec2<usize>) -> Self::Pixel) {
    self.iter_mut().for_each(|(p, xy)| {
      *p = writer(xy.into());
    });
  }
}

pub trait Texture2dInitAble: Texture2D {
  fn init_with(size: Size, pixel: Self::Pixel) -> Self;
  /// Opt in use a fast allocation call,
  /// use this function to get better performance.
  fn init_not_care(size: Size) -> Self;
}

/// Not all texture storage container has continues memory,
/// use this trait to get under laying buffer for GPU resource uploading
pub trait BufferLikeTexture2D: Texture2D {
  fn as_byte(&self) -> &[u8];
}

pub trait Texture2DSampleAble: Texture2D {
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
    let corrected = position.map(address);
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
