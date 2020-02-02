use crate::component::UpdateCtx;
use crate::renderer::GUIRenderer;
use rendiation_util::*;

pub struct Event {}

pub trait Element<T> {
  fn render(&self, renderer: &mut GUIRenderer);
  fn event(&self, event: &Event, state: &mut T);
  // fn is_point_in(&self) -> bool;
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
  x: f32,
  y: f32,
}

pub struct Quad<C> {
  click_listeners: Vec<Box<dyn Fn(&Event, &mut C, &mut UpdateCtx)>>,
  pub quad: QuadLayout,
}

impl<C> Quad<C> {
  pub fn new() -> Self {
    Self {
      click_listeners: Vec::new(),
      quad: QuadLayout {
        width: 1.,
        height: 1.,
        x: 1.,
        y: 1.,
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

impl<T> Element<T> for Quad<T>{

  fn render(&self, renderer: &mut GUIRenderer){

  }
  fn event(&self, event: &Event, state: &mut T){

  }
}

pub struct Text {
  content: String,
}
