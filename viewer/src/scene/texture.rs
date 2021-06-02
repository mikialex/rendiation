use super::BindableResource;

pub struct SceneTexture2D {
  data: Box<dyn SceneTexture2dSource>,
  gpu: Option<SceneTexture2dGpu>,
}

impl SceneTexture2D {
  pub fn get_gpu_view(&mut self, device: &wgpu::Device) -> &wgpu::TextureView {
    todo!()
  }
}

pub trait SceneTexture2dSource {
  fn as_byte(&self) -> &[u8];
}

pub struct SceneTexture2dGpu {
  gpu: wgpu::Texture,
  view: wgpu::TextureView,
}

impl BindableResource for SceneTexture2dGpu {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(&self.view)
  }
}
