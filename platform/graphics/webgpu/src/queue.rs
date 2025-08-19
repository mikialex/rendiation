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

  pub fn submit_encoder(&self, encoder: GPUCommandEncoder) {
    let cmd = encoder.finish();
    self.inner.submit(std::iter::once(cmd.inner));
    cmd.on_submit.emit(&());
  }
}

impl Deref for GPUQueue {
  type Target = wgpu::Queue;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
