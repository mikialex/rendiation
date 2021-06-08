use super::{BindableResource, SceneTexture2dSource};

pub struct SceneTextureCube {
  data: Box<dyn SceneTexture2dSource>,
  gpu: Option<SceneTextureCubeGPU>,
}

pub struct SceneTextureCubeGPU {
  texture: wgpu::Texture,
  texture_view: wgpu::TextureView,
}

impl BindableResource for SceneTextureCubeGPU {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(&self.texture_view)
  }
}
