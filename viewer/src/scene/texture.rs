use super::BindableResource;

pub struct SceneTexture2D {
  data: Box<dyn SceneTexture2dSource>,
  gpu: Option<SceneTexture2dGpu>,
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
