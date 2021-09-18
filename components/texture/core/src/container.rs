use rendiation_texture_types::Size;

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
}
