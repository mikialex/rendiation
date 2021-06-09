use crate::renderer::{SceneTexture2dGpu, SceneTexture2dSource};

use super::{Scene, Texture2DHandle};

pub struct SceneTexture2D {
  data: Box<dyn SceneTexture2dSource>,
  gpu: Option<SceneTexture2dGpu>,
}

impl SceneTexture2D {
  pub fn get_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> &SceneTexture2dGpu {
    self
      .gpu
      .get_or_insert_with(|| SceneTexture2dGpu::create(&device, queue, self.data.as_ref()))
  }
}

impl Scene {
  pub fn add_texture2d(&mut self, texture: impl SceneTexture2dSource) -> Texture2DHandle {
    self.texture_2ds.insert(SceneTexture2D {
      data: Box::new(texture),
      gpu: None,
    })
  }
}
