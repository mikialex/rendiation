use crate::*;
use winit::event::*;

pub trait HotAreaProvider {
  fn is_point_in(&self, point: UIPosition) -> bool;
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

#[derive(Default)]
pub struct MouseUp;
pub type MouseUpHandler<T> = EventHandler<T, MouseUp>;
impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseUp {
  fn downcast_event(&mut self, event: &mut EventCtx, inner: &C) -> bool {
    if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.event) {
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

#[derive(Default)]
pub struct MouseMove;
pub type MouseMoveHandler<T> = EventHandler<T, MouseMove>;
impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseMove {
  fn downcast_event(&mut self, event: &mut EventCtx, inner: &C) -> bool {
    if let Some(position) = mouse_move(event.event) {
      if inner.is_point_in((position.x as f32, position.y as f32).into()) {
        return true;
      }
    }
    false
  }
}

pub struct MouseIn {
  is_mouse_in: bool,
}
impl Default for MouseIn {
  fn default() -> Self {
    Self { is_mouse_in: false }
  }
}
pub type MouseInHandler<T> = EventHandler<T, MouseIn>;
impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseIn {
  fn downcast_event(&mut self, event: &mut EventCtx, inner: &C) -> bool {
    if let Some(position) = mouse_move(event.event) {
      if inner.is_point_in((position.x as f32, position.y as f32).into()) {
        if !self.is_mouse_in {
          self.is_mouse_in = true;
          return true;
        }
        self.is_mouse_in = true;
      } else {
        self.is_mouse_in = false;
      }
    }
    false
  }
}

pub struct MouseOut {
  is_mouse_in: bool,
}
impl Default for MouseOut {
  fn default() -> Self {
    Self { is_mouse_in: false }
  }
}
pub type MouseOutHandler<T> = EventHandler<T, MouseOut>;
impl<C: HotAreaProvider> EventHandlerImpl<C> for MouseOut {
  fn downcast_event(&mut self, event: &mut EventCtx, inner: &C) -> bool {
    if let Some(position) = mouse_move(event.event) {
      if !inner.is_point_in((position.x as f32, position.y as f32).into()) {
        if self.is_mouse_in {
          self.is_mouse_in = false;
          return true;
        }
        self.is_mouse_in = false;
      } else {
        self.is_mouse_in = true;
      }
    }
    false
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

fn mouse_move(event: &Event<()>) -> Option<winit::dpi::PhysicalPosition<f64>> {
  window_event(event).and_then(|e| match e {
    WindowEvent::CursorMoved { position, .. } => Some(*position),
    _ => None,
  })
}
