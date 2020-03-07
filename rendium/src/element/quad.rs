use crate::element::RenderCtx;
use super::{Element, Message};
use crate::element::ElementState;
use crate::{event::Event, renderer::GUIRenderer};
use rendiation_math::*;

pub struct QuadLayout {
  width: f32,
  height: f32,
  x: f32,
  y: f32,
}

impl QuadLayout {
  pub fn is_point_in(&self, point: Vec2<f32>) -> bool {
    point.x >= self.x
      && point.y >= self.y
      && point.x <= self.x + self.width
      && point.y <= self.y + self.height
  }
}

pub struct Quad {
  pub quad: QuadLayout,
  element_state: ElementState,
}

impl Quad {
  pub fn new() -> Self {
    Self {
      quad: QuadLayout {
        width: 100.,
        height: 100.,
        x: 0.,
        y: 0.,
      },
      element_state: ElementState::new(),
    }
  }
}

impl Element for Quad {
  fn render(&self, renderer: &mut RenderCtx) {
    let r = &mut renderer.renderer;
    r.draw_rect(&mut renderer.backend,100.0, 200.0, 100.0, 100.0);
  }
  fn event(&self, event: &mut Message) {
    // decide if event need handled
  }
  fn get_element_state(&self) -> &ElementState {
    &self.element_state
  }
  fn is_point_in(&self, point: Vec2<f32>) -> bool {
    self.quad.is_point_in(point)
  }
}
