use crate::renderer::texture::ImageProvider;

pub struct ImageData {
  pub data: Vec<u8>,
  pub width: usize,
  pub height: usize,
}

impl ImageData {
  pub fn new(data: Vec<u8>, width: usize, height: usize) -> Self {
    ImageData {
      data,
      width,
      height,
    }
  }
}

impl ImageProvider for ImageData {
  fn get_size(&self) -> (u32, u32, u32) {
    (self.width as u32, self.height as u32, 1)
  }
  fn get_data(&self) -> &[u8] {
    &self.data
  }
}
