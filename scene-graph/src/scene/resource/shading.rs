use crate::{Index, ResourceManager, SceneGraphBackEnd};

pub struct SceneShadingDescriptor {
  pub vertex_shader_str: String,
  pub frag_shader_str: String,
  // .. blend state stuff
}

/// webgpu => pipeline
/// webgl => program
pub struct SceneShading<T: SceneGraphBackEnd> {
  index: Index,
  parameters: Vec<Option<Index>>,
  gpu: T::Shading,
}

impl<T: SceneGraphBackEnd> SceneShading<T> {
  pub fn get_gpu(&self) -> &T::Shading {
    &self.gpu
  }
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn create_shading(&mut self, shading: SceneShadingDescriptor) -> SceneShading<T> {
    todo!()
    // self.shadings.insert(shading)
  }

  pub fn get_shading_mut(&mut self, index: Index) -> &mut SceneShading<T> {
    self.shadings.get_mut(index).unwrap()
  }

  pub fn get_shading(&self, index: Index) -> &SceneShading<T> {
    self.shadings.get(index).unwrap()
  }

  pub fn delete_shading(&mut self, index: Index) {
    self.shadings.remove(index);
  }
}
