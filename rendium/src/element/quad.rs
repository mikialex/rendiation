use crate::element::ElementState;
use rendiation_math::*;
use crate::{renderer::GUIRenderer, event::Event};
use super::Element;

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

pub struct Quad<C> {
  listeners: Vec<Box<dyn Fn(&Event, &mut C)>>,
  pub quad: QuadLayout,
  element_state: ElementState,
}

impl<C> Quad<C> {
  pub fn new() -> Self {
    Self {
      listeners: Vec::new(),
      quad: QuadLayout {
        width: 100.,
        height: 100.,
        x: 0.,
        y: 0.,
      },
      element_state: ElementState::new()
    }
  }

  pub fn listener<T: Fn(&Event, &mut C) + 'static>(&mut self, func: T) {
    self.listeners.push(Box::new(func));
  }

  pub fn trigger_listener(&self, event: &Event, component_state: &mut C) {
    for listener in self.listeners.iter() {
      listener(event, component_state);
    }
  }
}

impl<T> Element<T> for Quad<T> {
  fn render(&self, renderer: &mut GUIRenderer) {
    renderer.draw_rect(0.0, 0.0, 0.0, 0.0);
  }
  fn event(&self, event: &Event, state: &mut T) {
    // decide if event need handled
  }
  fn get_element_state(&self) -> &ElementState {
    &self.element_state
  }
}
