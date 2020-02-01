use crate::renderer::GUIRenderer;
use rendiation_util::*;

pub struct Event {}

pub trait Element<T> {
  fn render(&self, renderer: &mut GUIRenderer);
  fn event(&self, event: &Event, state: &mut T);
  fn is_point_in(&self) -> bool;
}

pub struct ElementsTree<T> {
  elements: Tree<Box<dyn Element<T>>>,
}

impl<T> ElementsTree<T> {
  fn event(&self, event: &Event, state: &mut T) {}
}

pub struct QuadLayout {
  width: f32,
  height: f32,
  left_offset: f32,
  topL_offset: f32,
}

pub struct Quad<C> {
  click_listeners: Vec<Box<dyn Fn(&Event, &mut C, &mut UpdateCtx)>>,
  quad: QuadLayout,
}

pub struct UpdateCtx {}

impl<C> Quad<C> {
  pub fn new() -> Self {
    Self {
      click_listeners: Vec::new(),
      quad: QuadLayout {
        width: 1.,
        height: 1.,
        left_offset: 1.,
        topL_offset: 1.,
      },
    }
  }

  pub fn listener<T: Fn(&Event, &mut C, &mut UpdateCtx) + 'static>(&mut self, func: T) {
    self.click_listeners.push(Box::new(func));
  }

  pub fn trigger_listener(&self, event: &Event, component_state: &mut C, ctx: &mut UpdateCtx) {
    for listener in self.click_listeners.iter() {
      listener(event, component_state, ctx);
    }
  }
}

pub struct Text {
  content: String,
}
