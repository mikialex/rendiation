pub mod forward;
pub use forward::*;

pub mod list;
pub use list::*;

// pub mod copy_frame;
// pub use copy_frame::*;
// pub mod highlight;
// pub use highlight::*;
// pub mod background;
// pub use background::*;
pub mod utils;
use rendiation_webgpu::GPURenderPass;
pub use utils::*;

pub mod framework;
pub use framework::*;

pub struct SceneRenderPass<'a> {
  pass: GPURenderPass<'a>,
  // pub pass_gpu_cache: &'a PassGPUDataCache,
}

impl<'a> std::ops::Deref for SceneRenderPass<'a> {
  type Target = GPURenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl<'a> std::ops::DerefMut for SceneRenderPass<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}
