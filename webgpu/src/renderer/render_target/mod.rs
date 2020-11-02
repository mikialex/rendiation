pub mod custom;
pub mod screen;

use crate::{WGPURenderPassBuilder, WGPURenderer};
pub use custom::*;
use rendiation_ral::{RenderTargetFormatsInfo, TargetStates};
pub use screen::*;

pub trait RenderTargetAble: TargetInfoProvider {
  fn create_render_pass_builder(&self) -> WGPURenderPassBuilder;
  fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize));
  fn get_size(&self) -> (usize, usize);
}

pub trait TargetInfoProvider {
  fn create_target_states(&self) -> TargetStates;
  fn provide_format_info(&self) -> RenderTargetFormatsInfo;
}
