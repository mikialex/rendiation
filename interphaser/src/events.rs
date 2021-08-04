use std::rc::Rc;

use crate::*;
use rendiation_webgpu::GPU;
use winit::event::*;

pub struct EventCtx<'a> {
  pub event: &'a winit::event::Event<'a, ()>,
  pub states: &'a WindowState,
  pub gpu: Rc<GPU>,
}

pub struct EventHandler<T, X> {
  state: X,
  handler: Box<dyn Fn(&mut T)>,
}

impl<T, X: Default> EventHandler<T, X> {
  pub fn by(fun: impl Fn(&mut T) + 'static) -> Self {
    Self {
      state: Default::default(),
      handler: Box::new(fun),
    }
  }
}

impl<T, X: EventHandlerImpl<C>, C: Component<T>> ComponentAbility<T, C> for EventHandler<T, X> {
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    if self.state.downcast_event(event, inner) {
      (self.handler)(model)
    }
    inner.event(model, event);
  }
}

impl<T, X, C: Presentable> PresentableAbility<C> for EventHandler<T, X> {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
    inner.render(builder);
  }
}

impl<T, X, C: LayoutAble> LayoutAbility<C> for EventHandler<T, X> {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutResult {
    inner.layout(constraint, ctx)
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    inner.set_position(position)
  }
}

impl<T, X, C: HotAreaProvider> HotAreaPassBehavior<C> for EventHandler<T, X> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    inner.is_point_in(point)
  }
}

pub trait EventHandlerImpl<C> {
  fn downcast_event(&mut self, event: &mut EventCtx, inner: &C) -> bool;
}

#[derive(Default)]
pub struct MouseDown;
pub type MouseDownHandler<T> = EventHandler<T, MouseDown>;
impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseDown {
  fn downcast_event(&mut self, event: &mut EventCtx, inner: &C) -> bool {
    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
      if inner.is_point_in(event.states.mouse_position) {
        return true;
      }
    }
    false
  }
}

pub struct Click {
  mouse_down: bool,
}
impl Default for Click {
  fn default() -> Self {
    Self { mouse_down: false }
  }
}

pub type ClickHandler<T> = EventHandler<T, Click>;

impl<C: HotAreaProvider> EventHandlerImpl<C> for Click {
  fn downcast_event(&mut self, event: &mut EventCtx, inner: &C) -> bool {
    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
      if inner.is_point_in(event.states.mouse_position) {
        self.mouse_down = true;
      }
    } else if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.event) {
      if self.mouse_down && inner.is_point_in(event.states.mouse_position) {
        return true;
      }
      self.mouse_down = false;
    }
    false
  }
}

pub trait HotAreaProvider {
  fn is_point_in(&self, point: UIPosition) -> bool;
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
