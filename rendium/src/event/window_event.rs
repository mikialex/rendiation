use crate::application::AppRenderCtx;
use core::any::Any;
use generational_arena::*;
use std::collections::HashMap;
use winit::event;
use winit::event::*;

pub struct EventCtx<'a, 'b, 'c, T> {
  pub event: winit::event::Event<()>, // todo use self event
  pub state: &'a mut T,
  pub render_ctx: &'b mut AppRenderCtx<'c>,
}

type ListenerContainer<AppState> = Arena<Box<dyn FnMut(&mut EventCtx<AppState>)>>;

struct Message<'a> {
  target: &'a mut dyn Any,
}

struct EventSession {
  listeners: Vec<Box<dyn FnMut(&mut Message)>>,
}

impl EventSession {
  pub fn new() -> Self {
    Self {
      listeners: Vec::new(),
    }
  }

  pub fn emit() {}

  pub fn add<T: FnMut(&mut Message) + 'static>(&mut self, listener: T) {
    self.listeners.push(Box::new(listener));
  }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum EventType {
  EventCleared,
  MouseDown,
  MouseMotion,
  MouseWheel,
  Resize,
}

pub struct WindowEventSession<AppState> {
  raw_listeners: ListenerContainer<AppState>,
  fixed_listeners: HashMap<EventType, ListenerContainer<AppState>>,
}

fn emit_listener<AppState>(
  listeners: Option<&mut ListenerContainer<AppState>>,
  event: &mut EventCtx<AppState>,
) {
  if let Some(listeners) = listeners {
    listeners.iter_mut().for_each(|(_, f)| f(event))
  }
}

impl<AppState> WindowEventSession<AppState> {
  pub fn add_listener_raw<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    func: T,
  ) -> Index {
    self.raw_listeners.insert(Box::new(func))
  }

  pub fn add_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    event_type: EventType,
    func: T,
  ) -> Index {
    let container = self
      .fixed_listeners
      .entry(event_type)
      .or_insert_with(|| Arena::new());
    container.insert(Box::new(func))
  }

  pub fn new() -> Self {
    Self {
      raw_listeners: Arena::new(),
      fixed_listeners: HashMap::new(),
    }
  }

  pub fn event(
    &mut self,
    event: winit::event::Event<()>,
    s: &mut AppState,
    renderer: &mut AppRenderCtx,
  ) {
    let mut event_ctx = EventCtx {
      event: event.clone(),
      state: s,
      render_ctx: renderer,
    };

    emit_listener(Some(&mut self.raw_listeners), &mut event_ctx);

    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          emit_listener(
            self.fixed_listeners.get_mut(&EventType::Resize),
            &mut event_ctx,
          );
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => emit_listener(
              self.fixed_listeners.get_mut(&EventType::MouseDown),
              &mut event_ctx,
            ),
            ElementState::Released => (),
          },
          MouseButton::Right => match state {
            ElementState::Pressed => (),
            ElementState::Released => (),
          },
          _ => {}
        },
        WindowEvent::MouseWheel { delta, .. } => {
          if let MouseScrollDelta::LineDelta(x, y) = delta {
            emit_listener(
              self.fixed_listeners.get_mut(&EventType::MouseWheel),
              &mut event_ctx,
            );
          }
        }
        WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => emit_listener(
          self.fixed_listeners.get_mut(&EventType::MouseMotion),
          &mut event_ctx,
        ),
        _ => (),
      },
      event::Event::EventsCleared => emit_listener(
        self.fixed_listeners.get_mut(&EventType::EventCleared),
        &mut event_ctx,
      ),

      DeviceEvent => {}
    }
  }
}
