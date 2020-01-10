use rendiation::*;
use rendiation::ImageProvider;

pub struct Texture {
  image: ImageData,
  gpu_texture: WGPUTexture
}

impl Texture{
  pub fn new<R: Renderer>(image: ImageData, renderer: &mut WGPURenderer<R>) -> Self{
    let gpu_texture = WGPUTexture::new(&renderer.device, &mut renderer.encoder, &image);
    Texture{
      image, 
      gpu_texture
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
