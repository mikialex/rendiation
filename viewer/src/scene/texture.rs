use super::BindableResource;

pub struct SceneTexture2D {
  data: Vec<u8>,
  gpu: wgpu::Texture,
  view: wgpu::TextureView,
}

impl BindableResource for SceneTexture2D {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(&self.view)
  }
}
