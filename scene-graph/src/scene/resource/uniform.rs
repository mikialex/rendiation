use crate::{Index, ResourceManager, SceneGraphBackEnd};

pub struct Uniform<T: SceneGraphBackEnd> {
  index: Index,
  gpu: T::UniformBuffer,
}

impl<T: SceneGraphBackEnd> Uniform<T> {
  pub fn index(&self) -> Index {
    self.index
  }

  pub fn gpu(&self) -> &T::UniformBuffer {
    &self.gpu
  }
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_uniform(&mut self, gpu: T::UniformBuffer) -> &mut Uniform<T> {
    let wrapped = Uniform {
      index: Index::from_raw_parts(0, 0),
      gpu,
    };
    let index = self.uniforms.insert(wrapped);
    let u = self.get_uniform_mut(index);
    u.index = index;
    u
  }

  pub fn get_uniform_mut(&mut self, index: Index) -> &mut Uniform<T> {
    self.uniforms.get_mut(index).unwrap()
  }

  pub fn get_uniform(&self, index: Index) -> &Uniform<T> {
    self.uniforms.get(index).unwrap()
  }

  pub fn delete_uniform(&mut self, index: Index) {
    self.uniforms.remove(index);
  }
}
