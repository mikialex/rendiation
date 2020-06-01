use crate::{Index, ResourceUpdateCtx, SceneGraphBackEnd};
use std::any::Any;

pub struct Uniform<T: SceneGraphBackEnd> {
  index: Index,
  data: Box<dyn Any>,
  gpu: T::UniformBuffer,
}

impl<T: SceneGraphBackEnd> Uniform<T> {
  pub fn index(&self) -> Index {
    self.index
  }

  pub fn gpu(&self) -> &T::UniformBuffer {
    &self.gpu
  }

  pub fn mutate<V: 'static>(&mut self, update_ctx: &mut ResourceUpdateCtx) -> &V {
    update_ctx.notify_uniform_update(self.index);
    self.data.downcast_mut::<V>().unwrap()
  }
}
