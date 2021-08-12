mod layout;
pub use layout::*;

mod rendering;
pub use rendering::*;

use crate::*;
use rendiation_webgpu::GPU;
use std::{any::Any, rc::Rc};

pub trait Component<T, S: System = DefaultSystem> {
  fn event(&mut self, _model: &mut T, _event: &mut S::EventCtx<'_>) {}

  fn update(&mut self, _model: &T, _ctx: &mut S::UpdateCtx<'_>) {}
}

pub trait System {
  type EventCtx<'a>;
  type UpdateCtx<'a>;
}

pub struct DefaultSystem {}

impl System for DefaultSystem {
  type EventCtx<'a> = EventCtx<'a>;
  type UpdateCtx<'a> = UpdateCtx;
}

pub struct UpdateCtx {
  pub time_stamp: u64,
  pub layout_changed: bool, // todo private
}

impl UpdateCtx {
  pub fn request_layout(&mut self) {
    self.layout_changed = true;
  }
}

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub custom_event: CustomEventCtx,
  pub states: &'a WindowState,
  pub gpu: Rc<GPU>,
}

pub trait UIComponent<T>: Component<T> + Presentable + LayoutAble + 'static {}
impl<X, T> UIComponent<T> for X where X: Component<T> + Presentable + LayoutAble + 'static {}

pub struct CustomEventCtx {
  events: Vec<Box<dyn Any>>,
  drain_index: Vec<usize>,
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
