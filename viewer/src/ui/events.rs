use crate::WindowState;

use super::{Component, ComponentAbility, UIPosition};
use winit::event::*;

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub states: &'a WindowState,
}

struct EventHandler<T> {
  handler: Box<dyn Fn(&mut T)>,
}

impl<T, C: Component<T>> ComponentAbility<T, C> for EventHandler<T> {
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {}
}

struct ClickHandler<T> {
  mouse_down: bool,
  handler: Box<dyn Fn(&mut T)>,
}

pub trait HotAreaProvider {
  fn is_point_in(&self, point: UIPosition) -> bool;
}

impl<T, C> ComponentAbility<T, C> for ClickHandler<T>
where
  C: Component<T> + HotAreaProvider,
{
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
      if inner.is_point_in(event.states.mouse_position) {
        (self.handler)(model)
      }
    }
    inner.event(model, event);
  }
}

fn window_event<'a>(event: &'a Event<()>) -> Option<&'a WindowEvent<'a>> {
  match event {
    Event::WindowEvent { event, .. } => Some(event),
    _ => None,
  }
}

fn mouse(event: &Event<()>) -> Option<(MouseButton, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::MouseInput { state, button, .. } => Some((*button, *state)),
    _ => None,
  })
}
