pub mod quad;
pub mod fragment;
pub use fragment::*;
use core::any::Any;
use rendiation_math::*;
pub use quad::*;
use crate::{event::Event, renderer::GUIRenderer};
// pub mod tree;

pub struct Message<'a> {
  target: &'a mut dyn Any,
}

pub struct RenderCtx<'a> {
  renderer: &'a mut GUIRenderer
}

pub trait Element {
  fn render(&self, renderer: &mut RenderCtx);
  fn event(&self, event: &mut Message);
  fn get_element_state(&self) -> &ElementState;
  fn is_point_in(&self, point: Vec2<f32>) -> bool;
}

pub struct ElementState{
  is_active: bool,
  is_hover: bool,
  is_focus: bool,
  z_index: i32,
}

impl ElementState{
  pub fn new() -> Self{
    Self {
      is_active: false,
      is_hover: false,
      is_focus: false,
      z_index: 0,
    }
  }
}
