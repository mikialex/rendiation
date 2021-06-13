use crate::renderer::{BindableResource, SceneTexture2dGpu, SceneTexture2dSource};

use super::{MaterialHandle, ResourcePair, Scene, Texture2DHandle};

pub struct SceneTexture2D {
  data: Box<dyn SceneTexture2dSource>,
  res: SceneTexture2DResource,
}

pub struct SceneTexture2DResource {
  gpu: Option<SceneTexture2dGpu>,
  used_by: Vec<MaterialHandle>,
}

impl SceneTexture2DResource {
  pub fn as_material_bind(&mut self, material: MaterialHandle) -> wgpu::BindingResource {
    self.used_by.push(material);
    self.gpu.as_ref().unwrap().as_bindable()
  }
}

impl ResourcePair for SceneTexture2D {
  type Data = Box<dyn SceneTexture2dSource>;
  type Resource = SceneTexture2DResource;
  fn data(&self) -> &Self::Data {
    &self.data
  }
  fn resource(&self) -> &Self::Resource {
    &self.res
  }
  fn data_mut(&mut self) -> &mut Self::Data {
    &mut self.data
  }
  fn resource_mut(&mut self) -> &mut Self::Resource {
    &mut self.res
  }
}

impl SceneTexture2D {
  pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
    self
      .res
      .gpu
      .get_or_insert_with(|| SceneTexture2dGpu::create(&device, queue, self.data.as_ref()));
  }
}

impl Scene {
  pub fn add_texture2d(&mut self, texture: impl SceneTexture2dSource) -> Texture2DHandle {
    self.texture_2ds.insert(SceneTexture2D {
      data: Box::new(texture),
      res: SceneTexture2DResource {
        gpu: None,
        used_by: Vec::new(),
      },
    })
  }
}
