use crate::{Index, SceneGraphBackEnd};

pub struct SceneTexture<T: SceneGraphBackEnd> {
  index: Index,
  gpu: T::UniformBuffer,
}

impl<T: SceneGraphBackEnd> SceneTexture<T> {
  pub fn index(&self) -> Index {
    self.index
  }

  pub fn gpu(&self) -> &T::UniformBuffer {
    &self.gpu
  }

  pub fn gpu_mut(&mut self) -> &mut T::UniformBuffer {
    &mut self.gpu
  }
}
