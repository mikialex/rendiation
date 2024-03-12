use std::ops::Deref;

use winit::{
  event::*,
  keyboard::{KeyCode, PhysicalKey},
};

use crate::*;

pub struct EventHandler<X: EventHandlerType> {
  state: X,
  pub events: EventSource<X::Event>,
}

impl<T: EventHandlerType> Default for EventHandler<T> {
  fn default() -> Self {
    Self {
      state: Default::default(),
      events: Default::default(),
    }
  }
}

impl<X: EventHandlerType> EventHandler<X> {
  pub fn any_triggered() -> (Self, impl Stream<Item = ()>) {
    let event = Self::default();
    let stream = event.any_triggered();
    (event, stream)
  }

  pub fn on(f: impl FnMut(&X::Event) -> bool + 'static + Send + Sync) -> Self {
    let event = Self::default();
    event.on(f);
    event
  }
}

impl<X: EventHandlerType> Deref for EventHandler<X> {
  type Target = EventSource<X::Event>;
  fn deref(&self) -> &Self::Target {
    &self.events
  }
}

pub trait EventHandlerLike<C> {
  fn handle_event(&mut self, event: &mut EventCtx, inner: &mut C);
  fn should_handle_in_bubble(&self) -> bool;
}

pub struct EventHandlerGroup<C> {
  before_handlers: Vec<Box<dyn EventHandlerLike<C>>>,
  after_handlers: Vec<Box<dyn EventHandlerLike<C>>>,
}

impl<C> Default for EventHandlerGroup<C> {
  fn default() -> Self {
    Self {
      before_handlers: Default::default(),
      after_handlers: Default::default(),
    }
  }
}

impl<C> EventHandlerGroup<C> {
  #[must_use]
  pub fn with(mut self, handler: impl EventHandlerLike<C> + 'static) -> Self {
    if handler.should_handle_in_bubble() {
      self.after_handlers.push(Box::new(handler));
    } else {
      self.before_handlers.push(Box::new(handler));
    }
    self
  }
}

impl<C> Stream for EventHandlerGroup<C> {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

impl<C: View> ViewNester<C> for EventHandlerGroup<C> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C) {
    match detail {
      ViewRequest::Event(event) => {
        self
          .before_handlers
          .iter_mut()
          .for_each(|handler| handler.handle_event(event, inner));

        inner.event(event);

        self
          .after_handlers
          .iter_mut()
          .for_each(|handler| handler.handle_event(event, inner));
      }
      _ => inner.request(detail),
    }
  }
}
impl<X: EventHandlerType> Stream for EventHandler<X> {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

impl<C, X: EventHandlerImpl<C>> EventHandlerLike<C> for EventHandler<X> {
  fn handle_event(&mut self, event: &mut EventCtx, inner: &mut C) {
    if let Some(e) = self.state.process_event(event, inner) {
      self.events.emit(&e)
    }
  }
  fn should_handle_in_bubble(&self) -> bool {
    self.state.should_handle_in_bubble()
  }
}

impl<C: View, X: EventHandlerImpl<C>> ViewNester<C> for EventHandler<X> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut C) {
    match detail {
      ViewRequest::Event(event) => {
        if !self.state.should_handle_in_bubble() {
          self.handle_event(event, inner);
        }

        inner.event(event);

        if self.state.should_handle_in_bubble() {
          self.handle_event(event, inner);
        }
      }
      _ => inner.request(detail),
    }
  }
}

pub trait EventHandlerType: Default {
  type Event: 'static;
}

pub trait EventHandlerImpl<C>: EventHandlerType {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event>;
  fn should_handle_in_bubble(&self) -> bool {
    false
  }
}

#[derive(Default)]
pub struct MouseDown;
pub type MouseDownHandler = EventHandler<MouseDown>;
impl EventHandlerType for MouseDown {
  type Event = ();
}
impl<C: View> EventHandlerImpl<C> for MouseDown {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event> {
    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
      if inner.hit_test(event.states.mouse_position) {
        return Some(());
      }
    }
    None
  }
}

#[derive(Default)]
pub struct MouseUp;
pub type MouseUpHandler = EventHandler<MouseUp>;
impl EventHandlerType for MouseUp {
  type Event = ();
}
impl<C: View> EventHandlerImpl<C> for MouseUp {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event> {
    if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.event) {
      if inner.hit_test(event.states.mouse_position) {
        return Some(());
      }
    }
    None
  }
}

#[derive(Default)]
pub struct Click {
  mouse_down: bool,
}

pub type ClickHandler = EventHandler<Click>;
impl EventHandlerType for Click {
  type Event = ();
}
impl<C: View> EventHandlerImpl<C> for Click {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event> {
    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
      if inner.hit_test(event.states.mouse_position) {
        self.mouse_down = true;
      }
    } else if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.event) {
      if self.mouse_down && inner.hit_test(event.states.mouse_position) {
        self.mouse_down = false;
        return Some(());
      }
    }
    None
  }
}

#[derive(Default)]
pub struct MouseMove;
pub type MouseMoveHandler = EventHandler<MouseMove>;
impl EventHandlerType for MouseMove {
  type Event = ();
}
impl<C: View> EventHandlerImpl<C> for MouseMove {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event> {
    if let Some(position) = mouse_move(event.event) {
      if inner.hit_test((position.x as f32, position.y as f32).into()) {
        return Some(());
      }
    }
    None
  }
}

#[derive(Default)]
pub struct MouseIn {
  is_mouse_in: bool,
}

pub type MouseInHandler = EventHandler<MouseIn>;
impl EventHandlerType for MouseIn {
  type Event = ();
}
impl<C: View> EventHandlerImpl<C> for MouseIn {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event> {
    if let Some(position) = mouse_move(event.event) {
      if inner.hit_test((position.x as f32, position.y as f32).into()) {
        if !self.is_mouse_in {
          self.is_mouse_in = true;
          return Some(());
        }
        self.is_mouse_in = true;
      } else {
        self.is_mouse_in = false;
      }
    }
    None
  }
}

#[derive(Default)]
pub struct MouseOut {
  is_mouse_in: bool,
}
pub type MouseOutHandler = EventHandler<MouseOut>;
impl EventHandlerType for MouseOut {
  type Event = ();
}
impl<C: View> EventHandlerImpl<C> for MouseOut {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event> {
    if let Some(position) = mouse_move(event.event) {
      if !inner.hit_test((position.x as f32, position.y as f32).into()) {
        if self.is_mouse_in {
          self.is_mouse_in = false;
          return Some(());
        }
        self.is_mouse_in = false;
      } else {
        self.is_mouse_in = true;
      }
    }
    None
  }
}

// these downcast utils below is useful for downstream crates because they shouldn't care about
// impl details so they are public and export, we should consider warp them in a namespace in the
// future to prevent potential name collisions

pub fn window_event<'a>(event: &'a Event<()>) -> Option<&'a WindowEvent> {
  match event {
    Event::WindowEvent { event, .. } => Some(event),
    _ => None,
  }
}

pub fn mouse(event: &Event<()>) -> Option<(MouseButton, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::MouseInput { state, button, .. } => Some((*button, *state)),
    _ => None,
  })
}

pub fn keyboard(event: &Event<()>) -> Option<(Option<KeyCode>, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::KeyboardInput {
      event: KeyEvent {
        physical_key,
        state,
        ..
      },
      ..
    } => Some((
      match physical_key {
        PhysicalKey::Code(code) => Some(*code),
        _ => None,
      },
      *state,
    )),
    _ => None,
  })
}

pub fn mouse_move(event: &Event<()>) -> Option<winit::dpi::PhysicalPosition<f64>> {
  window_event(event).and_then(|e| match e {
    WindowEvent::CursorMoved { position, .. } => Some(*position),
    _ => None,
  })
}

#[derive(Default)]
pub struct Drag {
  mouse_down: bool,
}

#[derive(Clone, Copy)]
pub enum DragEvent {
  StartDrag,
  EndDrag,
  Dragging,
}

impl EventHandlerType for Drag {
  type Event = DragEvent;
}
impl<C: View> EventHandlerImpl<C> for Drag {
  fn process_event(&mut self, event: &mut EventCtx, inner: &mut C) -> Option<Self::Event> {
    if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event.event) {
      if inner.hit_test(event.states.mouse_position) {
        self.mouse_down = true;
        return Some(DragEvent::StartDrag);
      }
    } else if let Some((MouseButton::Left, ElementState::Released)) = mouse(event.event) {
      if self.mouse_down {
        self.mouse_down = false;
        return Some(DragEvent::EndDrag);
      }
    } else if mouse_move(event.event).is_some() && self.mouse_down {
      return Some(DragEvent::Dragging);
    }
    None
  }
}
