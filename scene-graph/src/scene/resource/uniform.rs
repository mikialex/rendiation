use crate::{Index, ResourceManager, SceneGraphBackEnd, ResouceWrap};

// pub struct Uniform<T: SceneGraphBackEnd> {
//   index: Index,
//   gpu: T::UniformBuffer,
// }

// impl<T: SceneGraphBackEnd> Uniform<T> {
//   pub fn index(&self) -> Index {
//     self.index
//   }

//   pub fn gpu(&self) -> &T::UniformBuffer {
//     &self.gpu
//   }

//   pub fn gpu_mut(&mut self) -> &mut T::UniformBuffer {
//     &mut self.gpu
//   }
// }

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_uniform(&mut self, gpu: T::UniformBuffer) -> &mut ResouceWrap<T::UniformBuffer> {
    ResouceWrap::new_wrap(&mut self.uniforms, gpu)
  }

  pub fn get_uniform_mut(&mut self, index: Index) -> &mut ResouceWrap<T::UniformBuffer> {
    self.uniforms.get_mut(index).unwrap()
  }

  pub fn get_uniform(&self, index: Index) -> &ResouceWrap<T::UniformBuffer> {
    self.uniforms.get(index).unwrap()
  }

  pub fn delete_uniform(&mut self, index: Index) {
    self.uniforms.remove(index);
  }
}
