use crate::image_data::ImageData;
use crate::renderer::WGPURenderer;
use crate::WGPUTexture;

pub trait ImageProvider {
  fn get_size(&self) -> (u32, u32, u32);
  fn get_data(&self) -> &[u8];
}

pub struct Texture2D<T: ImageProvider = ImageData> {
  data: T,
  gpu: WGPUTexture,
}

impl<T: ImageProvider> Texture2D<T> {
  pub fn new(image: T, renderer: &mut WGPURenderer) -> Self {
    let gpu = WGPUTexture::new(&renderer.device, &mut renderer.encoder, &image);
    Texture2D { data: image, gpu }
  }
}
