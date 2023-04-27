use crate::*;

mod wrapper;
pub use wrapper::*;

mod content_system;
pub use content_system::*;

mod scene_system;
pub use scene_system::*;

mod global_system;
pub use global_system::*;

#[derive(Clone)]
pub struct ResourceGPUCtx {
  pub device: GPUDevice,
  pub queue: GPUQueue,
  pub mipmap_gen: Rc<RefCell<MipMapTaskManager>>,
}

impl ResourceGPUCtx {
  pub fn new(gpu: &GPU, mipmap_gen: Rc<RefCell<MipMapTaskManager>>) -> Self {
    Self {
      device: gpu.device.clone(),
      queue: gpu.queue.clone(),
      mipmap_gen,
    }
  }
}
