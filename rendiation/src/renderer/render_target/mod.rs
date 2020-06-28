pub mod custom;
pub mod screen;
pub mod target_state;

use crate::{WGPURenderPassBuilder, WGPURenderer};
pub use custom::*;
pub use screen::*;
pub use target_state::*;

pub trait RenderTargetAble: TargetStatesProvider {
  fn create_render_pass_builder(&self) -> WGPURenderPassBuilder;
  fn resize(&mut self, renderer: &WGPURenderer, size: (usize, usize));
}

pub trait TargetStatesProvider {
  fn create_target_states(&self) -> TargetStates;
}
