use crate::{Index, SceneGraphBackEnd};

pub struct SceneTexture<T: SceneGraphBackEnd> {
  index: Index,
  gpu: T::UniformBuffer,
}

