use rendiation::WGPURenderer;
use super::Message;
use crate::element::Element;
use crate::element::ElementState;
use crate::event::Event;
use crate::{Quad, renderer::GUIRenderer, RenderCtx};
use core::any::Any;
use rendiation_math::Vec2;

pub struct EventHub {
  listeners: Vec<Box<dyn FnMut(&mut Message)>>,
}

impl EventHub {
  pub fn new() -> Self {
    EventHub {
      listeners: Vec::new(),
    }
  }

  pub fn add<T: FnMut(&mut Message) + 'static>(&mut self, listener: T) {
    self.listeners.push(Box::new(listener));
  }
}

pub struct ElementFragment {
  elements: Vec<Box<dyn Element>>,
  elements_event: Vec<Vec<usize>>,
  events: EventHub,
}

impl ElementFragment {
  pub fn new() -> Self {

    let a = Quad::new();

    let mut fragment = ElementFragment {
      elements: Vec::new(),
      elements_event: Vec::new(),
      events: EventHub::new(),
    };
    fragment.add_element(a);
    fragment
  }

  pub fn add_element<T: Element + 'static>(&mut self, element: T) -> usize {
    let boxed = Box::new(element);
    let index = self.elements.len();
    self.elements.push(boxed);
    index
  }

  pub fn add_event_listener<T: FnMut(&mut Message) + 'static>(
    &mut self,
    element_index: usize,
    listener: T,
  ) -> usize {
    todo!();
  }

  pub fn render(&self, renderer: &mut WGPURenderer, gui_renderer: &GUIRenderer) {
    let mut ctx =  RenderCtx {
      renderer: gui_renderer,
      backend: renderer,
    };
    for element in &self.elements {
      element.render(&mut ctx)
    }
  }

  pub fn event<T: Any>(&mut self, payload: &mut T, event: Event, point: Vec2<f32>) {
    // todo!();
    let payload_any = payload as &mut dyn Any;
    let mut message = Message {
      target: payload_any,
    };
    for element in &mut self.elements {
      if element.is_point_in(point) {
        element.event(&mut message)
      }
    }
  }
}
