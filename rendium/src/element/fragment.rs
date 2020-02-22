use super::Message;
use crate::element::Element;
use crate::element::ElementState;
use crate::event::Event;
use crate::renderer::GUIRenderer;
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
    ElementFragment {
      elements: Vec::new(),
      elements_event: Vec::new(),
      events: EventHub::new(),
    }
  }

  pub fn add_element<T: Element>(&mut self, element: T) -> usize {
    todo!();
  }

  pub fn add_event_listener<T: FnMut(&mut Message) + 'static>(
    &mut self,
    element_index: usize,
    listener: T,
  ) -> usize {
    todo!();
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
