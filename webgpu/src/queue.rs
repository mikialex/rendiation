use std::{ops::Deref, rc::Rc};

#[derive(Clone)]
pub struct GPUQueue {
  inner: Rc<wgpu::Queue>,
}

impl GPUQueue {
  pub fn new(queue: wgpu::Queue) -> Self {
    Self {
      inner: Rc::new(queue),
    }
  }
}

impl Deref for GPUQueue {
  type Target = wgpu::Queue;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
