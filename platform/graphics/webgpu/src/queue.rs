use crate::*;

#[derive(Clone)]
pub struct GPUQueue {
  inner: Arc<wgpu::Queue>,
}

impl GPUQueue {
  pub fn new(queue: wgpu::Queue) -> Self {
    Self {
      inner: Arc::new(queue),
    }
  }
}

impl Deref for GPUQueue {
  type Target = wgpu::Queue;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
