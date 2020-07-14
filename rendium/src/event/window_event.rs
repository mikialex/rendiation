use crate::application::AppRenderCtx;
// use core::any::Any;
use arena::*;
use std::collections::HashMap;
use winit::event;
use winit::event::*;

pub struct EventCtx<'a, 'b, 'c, T> {
  pub event: &'a winit::event::Event<'b, ()>,
  pub state: &'a mut T,
  pub render_ctx: &'a mut AppRenderCtx<'c>,
  pub event_update_ctx: &'a mut WindowEventSessionUpdateCtx<T>,
}

type ListenerStorage<T> = Box<dyn FnMut(&mut EventCtx<T>)>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum EventType {
  EventCleared,
  MouseDown,
  MouseMotion,
  MouseWheel,
  Resize,
  Raw,
}

pub struct WindowEventSessionUpdateCtx<T> {
  to_remove: Vec<WindowEventSessionRemoveToken<T>>,
  to_add: WindowEventSessionData<T>,
}

impl<T> WindowEventSessionUpdateCtx<T> {
  pub fn new() -> Self {
    Self {
      to_remove: Vec::new(),
      to_add: WindowEventSessionData::new(),
    }
  }
}

pub struct WindowEventSessionData<T> {
  listeners: Arena<ListenerStorage<T>>, // real listener storage
  listeners_tags: HashMap<EventType, Arena<Handle<ListenerStorage<T>>>>,
}

impl<T> WindowEventSessionData<T> {
  pub fn new() -> Self {
    Self {
      listeners: Arena::new(),
      listeners_tags: HashMap::new(),
    }
  }
  pub fn add_listener_raw<U: FnMut(&mut EventCtx<T>) + 'static>(
    &mut self,
    func: U,
  ) -> WindowEventSessionRemoveToken<T> {
    self.add_listener(EventType::Raw, func)
  }

  pub fn add_listener<U: FnMut(&mut EventCtx<T>) + 'static>(
    &mut self,
    event_type: EventType,
    func: U,
  ) -> WindowEventSessionRemoveToken<T> {
    let id = self.listeners.insert(Box::new(func));

    let container = self
      .listeners_tags
      .entry(event_type)
      .or_insert_with(|| Arena::new());
    let container_id = container.insert(id);

    (event_type, container_id)
  }

  pub fn remove_listener(&mut self, id: WindowEventSessionRemoveToken<T>) {
    let listeners = &mut self.listeners;
    self
      .listeners_tags
      .get(&id.0)
      .map(|a| a.get(id.1).map(|l_id| listeners.remove(*l_id)));
  }

  pub fn emit(&mut self, ty: EventType, event: &mut EventCtx<T>) {
    let listeners = &mut self.listeners;
    self.listeners_tags.get(&ty).map(|a| {
      a.iter().for_each(|(_, h)| {
        listeners.get_mut(*h).unwrap()(event);
      })
    });
  }
}

pub struct WindowEventSession<T> {
  active: WindowEventSessionData<T>,
  update_ctx: WindowEventSessionUpdateCtx<T>,
}

pub type WindowEventSessionRemoveToken<T> = (EventType, Handle<Handle<ListenerStorage<T>>>);

impl<T> WindowEventSession<T> {
  // should i just deref to it?
  pub fn add_listener_raw<U: FnMut(&mut EventCtx<T>) + 'static>(
    &mut self,
    func: U,
  ) -> WindowEventSessionRemoveToken<T> {
    self.active.add_listener_raw(func)
  }

  pub fn add_listener<U: FnMut(&mut EventCtx<T>) + 'static>(
    &mut self,
    event_type: EventType,
    func: U,
  ) -> WindowEventSessionRemoveToken<T> {
    self.active.add_listener(event_type, func)
  }

  pub fn remove_listener(&mut self, id: WindowEventSessionRemoveToken<T>) {
    self.active.remove_listener(id)
  }

  pub fn new() -> Self {
    Self {
      active: WindowEventSessionData::new(),
      update_ctx: WindowEventSessionUpdateCtx::new(),
    }
  }

  pub fn event(&mut self, event: &winit::event::Event<()>, s: &mut T, renderer: &mut AppRenderCtx) {
    let mut event_ctx = EventCtx {
      event: &event,
      state: s,
      render_ctx: renderer,
      event_update_ctx: &mut self.update_ctx,
    };

    self.active.emit(EventType::Raw, &mut event_ctx);

    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.active.emit(EventType::Resize, &mut event_ctx);
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => self.active.emit(EventType::MouseDown, &mut event_ctx),
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
            self.active.emit(EventType::MouseWheel, &mut event_ctx);
          }
        }
        // WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { .. } => self.active.emit(EventType::MouseMotion, &mut event_ctx),
        _ => (),
      },
      event::Event::MainEventsCleared => self.active.emit(EventType::EventCleared, &mut event_ctx),
      _ => {}
    }
  }
}
