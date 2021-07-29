use std::rc::Rc;

use crate::*;
use rendiation_webgpu::GPU;
use winit::event::*;

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub states: &'a WindowState,
  pub gpu: Rc<GPU>,
}

pub struct EventHandler<T> {
  handler: Box<dyn Fn(&mut T)>,
}

impl<T, C: Component<T>> ComponentAbility<T, C> for EventHandler<T> {
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {}
}

pub struct ClickHandler<T> {
  mouse_down: bool,
  handler: Box<dyn FnMut(&mut T)>,
}

impl<T> ClickHandler<T> {
  pub fn by(handler: impl FnMut(&mut T) + 'static) -> Self {
    Self {
      mouse_down: false,
      handler: Box::new(handler),
    }
  }
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

impl<T, C: Presentable> PresentableAbility<C> for ClickHandler<T> {
  fn render(&self, builder: &mut PresentationBuilder, inner: &C) {
    inner.render(builder);
  }
}

impl<T, C: LayoutAble> LayoutAbility<C> for ClickHandler<T> {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutSize {
    inner.layout(constraint, ctx)
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    inner.set_position(position)
  }
}

impl<T, C: HotAreaProvider> HotAreaPassBehavior<C> for ClickHandler<T> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    inner.is_point_in(point)
  }
}
