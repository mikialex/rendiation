use super::Message;
use crate::element::Element;
use crate::element::ElementState;
use crate::event::Event;
use crate::{renderer::GUIRenderer, Quad, RenderCtx};
use arena::*;
use core::any::Any;
use rendiation_algebra::Vec2;
use rendiation_webgpu::WGPURenderer;

pub struct EventHub {
  listeners: Arena<Box<dyn FnMut(&mut Message)>>,
}

impl EventHub {
  pub fn new() -> Self {
    EventHub {
      listeners: Arena::new(),
    }
  }

  pub fn add<T: FnMut(&mut Message) + 'static>(
    &mut self,
    listener: T,
  ) -> Handle<Box<dyn FnMut(&mut Message)>> {
    self.listeners.insert(Box::new(listener))
  }
}

pub struct ElementFragment {
  elements: Vec<Box<dyn Element>>,
  elements_event: Vec<Vec<usize>>,
  listener_element_index: Vec<usize>,
  events: EventHub,
}

impl ElementFragment {
  pub fn new() -> Self {
    // let a = Quad::new();

    // let mut b = Quad::new();

    // b.position(200., 200.);

    let mut fragment = ElementFragment {
      elements: Vec::new(),
      elements_event: Vec::new(),
      listener_element_index: Vec::new(),
      events: EventHub::new(),
    };
    // let a = fragment.add_element(a);
    // let b = fragment.add_element(b);
    // fragment.add_event_listener(a, |m|{
    //   println!("dd");
    // });
    fragment
  }

  pub fn add_element<T: Element + 'static>(&mut self, element: T) -> usize {
    let boxed = Box::new(element);
    let index = self.elements.len();
    self.elements.push(boxed);
    self.elements_event.push(vec![]);
    index
  }

  pub fn add_event_listener<T: FnMut(&mut Message) + 'static>(
    &mut self,
    element_index: usize,
    listener: T,
  ) -> usize {
    todo!()
    // let event_index = self.events.add(listener);
    // self.listener_element_index.push(element_index);
    // self.elements_event[element_index].push(event_index);
    // event_index
  }

  pub fn render(&self, renderer: &mut WGPURenderer, gui_renderer: &mut GUIRenderer) {
    let mut ctx = RenderCtx {
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
        element.event(&mut message);
        // self.elements_event[element.index()];
      }
    }
  }
}
