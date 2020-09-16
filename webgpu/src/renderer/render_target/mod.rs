pub mod custom;
pub mod screen;
pub mod target_state;

use crate::{WGPURenderPassBuilder, WGPURenderer};
pub use custom::*;
pub use screen::*;
pub use target_state::*;

pub trait RenderTargetAble: TargetInfoProvider {
  fn create_render_pass_builder(&self) -> WGPURenderPassBuilder;
  fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize));
  fn get_size(&self) -> (usize, usize);
}

pub trait TargetInfoProvider {
  fn create_target_states(&self) -> TargetStates;
  fn provide_format_info(&self) -> RenderTargetFormatsInfo;
}
