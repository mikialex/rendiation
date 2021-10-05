use rendiation_texture_types::Size;

use crate::Texture2D;

pub struct Texture2DBuffer<P> {
  data: Vec<P>,
  size: Size,
}

impl<P: Clone> Texture2DBuffer<P> {
  pub fn new(size: Size) -> Self
  where
    P: Default,
  {
    Self {
      data: vec![P::default(); size.area()],
      size,
    }
  }

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

  fn get(&self, position: rendiation_algebra::Vec2<usize>) -> &Self::Pixel {
    &self.data[position.y * usize::from(self.size.width) + position.x]
  }

  fn get_mut(&mut self, position: rendiation_algebra::Vec2<usize>) -> &mut Self::Pixel {
    &mut self.data[position.y * usize::from(self.size.width) + position.x]
  }

  fn size(&self) -> Size {
    self.size
  }
}
