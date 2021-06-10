use crate::renderer::{SceneTexture2dGpu, SceneTexture2dSource};

use super::{MaterialHandle, Scene, Texture2DHandle};

pub struct MaterialBindGroupUsed {
  material: MaterialHandle,
  texture: Texture2DHandle,
}

pub struct SceneTexture2D {
  data: Box<dyn SceneTexture2dSource>,
  gpu: Option<SceneTexture2dGpu>,
  used_by: Vec<MaterialHandle>,
}

impl SceneTexture2D {
  pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
    self
      .gpu
      .get_or_insert_with(|| SceneTexture2dGpu::create(&device, queue, self.data.as_ref()));
  }

  pub fn get_gpu(&self) -> &SceneTexture2dGpu {
    self.gpu.as_ref().unwrap()
  }
}

impl Scene {
  pub fn add_texture2d(&mut self, texture: impl SceneTexture2dSource) -> Texture2DHandle {
    self.texture_2ds.insert(SceneTexture2D {
      data: Box::new(texture),
      gpu: None,
      used_by: Vec::new(),
    })
  }
}
