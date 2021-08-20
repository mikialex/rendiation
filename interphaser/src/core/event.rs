use rendiation_webgpu::GPU;
use std::{any::Any, rc::Rc};

use crate::WindowState;

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub custom_event: CustomEventCtx,
  pub states: &'a WindowState,
  pub gpu: Rc<GPU>,
}

pub struct CustomEventCtx {
  events: Vec<Box<dyn Any>>,
  drain_index: Vec<usize>,
}

impl CustomEventCtx {
  pub fn push_event(&mut self, e: impl Any) {
    self.events.push(Box::new(e))
  }
}

impl Default for CustomEventCtx {
  fn default() -> Self {
    Self {
      events: Vec::with_capacity(0),
      drain_index: Vec::with_capacity(0),
    }
  }
}

pub struct CustomEventEmitter {
  events: Vec<Box<dyn Any>>,
}

impl CustomEventEmitter {
  pub fn emit(&mut self, e: impl Any) {
    self.events.push(Box::new(e))
  }
}

impl Default for CustomEventEmitter {
  fn default() -> Self {
    Self {
      events: Vec::with_capacity(0),
    }
  }
}

impl CustomEventCtx {
  pub fn consume_if_type_is<E: 'static>(&mut self) -> Option<&E> {
    self.consume_if(|_| true)
  }
  pub fn consume_if<E: 'static>(&mut self, predicate: impl Fn(&E) -> bool) -> Option<&E> {
    if let Some(index) = self.events.iter().position(|e| {
      if let Some(e) = e.downcast_ref::<E>() {
        if predicate(e) {
          return true;
        }
      }
      false
    }) {
      self.drain_index.push(index);
      self.events.get(index).unwrap().downcast_ref::<E>()
    } else {
      None
    }
  }

  pub(crate) fn update(&mut self) {
    self.drain_index.drain(..).for_each(|i| {
      self.events.swap_remove(i);
    })
  }

  pub(crate) fn merge(&mut self, emitter: CustomEventEmitter) {
    self.events.extend(emitter.events);
  }
}
