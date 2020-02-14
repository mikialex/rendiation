use crate::renderer::WGPURenderer;
use crate::{WGPUTexture, ImageProvider};

impl ImageProvider for image::DynamicImage {
    fn get_size(&self) -> (u32, u32, u32) {
        todo!()
    }
    fn get_data(&self) -> &[u8] {

        todo!()
    }
}
 
pub struct Texture2D<T: ImageProvider> {
    data: T,
    gpu: WGPUTexture
}

impl<T: ImageProvider> Texture2D<T>{
    pub fn new(image: T, renderer: &mut WGPURenderer) -> Self {
        let gpu = WGPUTexture::new(&renderer.device, &mut renderer.encoder, &image);
        Texture2D {
            data: image,
            gpu
        }
    }

    // pub fn get_gpu(&mut self, renderer: &mut WGPURenderer) {
        
    // }
}