use rendiation::*;
use rendiation::ImageProvider;

use std::sync::atomic::{AtomicUsize, Ordering};
static GLOBAL_TEXTURE_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Texture {
  image: ImageData,
  gpu_texture: WGPUTexture,
  id: usize,
  need_update: bool,
}

impl Texture{
  pub fn new(image: ImageData, renderer: &mut WGPURenderer) -> Self{
    let gpu_texture = WGPUTexture::new(&renderer.device, &mut renderer.encoder, &image);
    Texture{
      image, 
      gpu_texture,
      id: GLOBAL_TEXTURE_ID.fetch_add(1, Ordering::SeqCst),
      need_update: false,
    }
  }
}

pub struct ImageData {
  pub data: Vec<u8>,
  pub width: usize,
  pub height: usize,
}

impl ImageProvider for ImageData {
  fn get_size(&self) -> (u32, u32, u32) {
    (self.width as u32, self.height as u32, 1)
  }
  fn get_data(&self) -> &[u8] {
    &self.data
  }
}
