use rendiation_algebra::Vec2;
use rendiation_texture_types::Size;

use crate::{Texture2D, Texture2dInitAble};

pub struct Texture2DBuffer<P> {
  data: Vec<P>,
  size: Size,
}

impl<P: Clone> Texture2DBuffer<P> {
  pub fn size(&self) -> Size {
    self.size
  }

  pub fn as_buffer(&self) -> &[P] {
    self.data.as_slice()
  }

  pub fn as_byte_buffer(&self) -> &[u8] {
    unsafe { std::mem::transmute(self.data.as_slice()) }
  }
}

impl<P: Copy> Texture2D for Texture2DBuffer<P> {
  type Pixel = P;
  fn get(&self, position: impl Into<Vec2<usize>>) -> &Self::Pixel {
    let position = position.into();
    &self.data[position.y * usize::from(self.size.width) + position.x]
  }

  fn get_mut(&mut self, position: impl Into<Vec2<usize>>) -> &mut Self::Pixel {
    let position = position.into();
    &mut self.data[position.y * usize::from(self.size.width) + position.x]
  }

  fn size(&self) -> Size {
    self.size
  }
}

impl<P: Copy + Default> Texture2dInitAble for Texture2DBuffer<P> {
  fn init_with(size: Size, pixel: Self::Pixel) -> Self {
    Self {
      data: vec![pixel; size.area()],
      size,
    }
  }

  #[allow(clippy::uninit_vec)]
  fn init_not_care(size: Size) -> Self {
    let width = usize::from(size.width);
    let height = usize::from(size.height);
    let mut buffer = Vec::with_capacity(width * height * 4);
    unsafe { buffer.set_len(width * height * 4) };
    Self { data: buffer, size }
  }
}
