use rendiation_algebra::Vec2;
use rendiation_texture_types::Size;
use wgpu_types::TextureFormat;

use crate::{Texture2D, Texture2dInitAble};

#[derive(Clone)]
pub struct Texture2DBuffer<P> {
  pub data: Vec<P>,
  pub size: Size,
}

impl<P> Texture2DBuffer<P> {
  pub fn from_raw(data: Vec<P>, size: Size) -> Self {
    assert_eq!(data.len(), size.area());
    Self { data, size }
  }
}

impl<T> core::fmt::Debug for Texture2DBuffer<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Texture2DBuffer")
      .field("data", &"raw data skipped")
      .field("size", &self.size)
      .finish()
  }
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

#[derive(Debug, Clone)]
pub struct GPUBufferImage {
  pub data: Vec<u8>,
  pub format: TextureFormat,
  pub size: Size,
}

pub fn create_padding_buffer(
  input: &[u8],
  step_read_byte_count: usize,
  step_pad_bytes: &[u8],
) -> Vec<u8> {
  // not checked the performance, maybe this could implemented in traditional way
  input
    .chunks(step_read_byte_count)
    .flat_map(|c| [c, step_pad_bytes])
    .flatten()
    .copied()
    .collect()
}
