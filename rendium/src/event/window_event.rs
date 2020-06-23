use crate::application::AppRenderCtx;
// use core::any::Any;
use arena::*;
use std::collections::HashMap;
use winit::event;
use winit::event::*;

pub struct EventCtx<'a, 'b, 'c, 'd, 'e, T> {
  pub event: &'e winit::event::Event<'d, ()>, // todo use self event
  pub state: &'a mut T,
  pub render_ctx: &'b mut AppRenderCtx<'c>,
}

type ListenerStorage<T> = Box<dyn FnMut(&mut EventCtx<T>)>;
// type ListenerContainer<T> = Arena<ListenerStorage<T>>;

// struct Message<'a> {
//   target: &'a mut dyn Any,
// }

// struct EventSession {
//   listeners: Vec<Box<dyn FnMut(&mut Message)>>,
// }

// impl EventSession {
//   pub fn new() -> Self {
//     Self {
//       listeners: Vec::new(),
//     }
//   }

//   pub fn emit() {}

//   pub fn add<T: FnMut(&mut Message) + 'static>(&mut self, listener: T) {
//     self.listeners.push(Box::new(listener));
//   }
// }

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum EventType {
  EventCleared,
  MouseDown,
  MouseMotion,
  MouseWheel,
  Resize,
  Raw,
}

pub struct WindowEventSession<AppState> {
  listeners: Arena<ListenerStorage<AppState>>, // real listener storage
  listeners_tags: HashMap<EventType, Arena<Handle<ListenerStorage<AppState>>>>,
}

fn emit_listener<AppState>(
  ty: EventType,
  listeners: &mut Arena<ListenerStorage<AppState>>,
  listeners_tags: &mut HashMap<EventType, Arena<Handle<ListenerStorage<AppState>>>>,
  event: &mut EventCtx<AppState>,
) {
  listeners_tags.get(&ty).map(|a| {
    a.iter().for_each(|(_, h)| {
      listeners.get_mut(*h).unwrap()(event);
    })
  });
}

pub type WindowEventSessionRemoveToken<T> = (EventType, Handle<Handle<ListenerStorage<T>>>);

impl<AppState> WindowEventSession<AppState> {
  pub fn add_listener_raw<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    func: T,
  ) -> WindowEventSessionRemoveToken<AppState> {
    self.add_listener(EventType::Raw, func)
  }

  pub fn add_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    event_type: EventType,
    func: T,
  ) -> WindowEventSessionRemoveToken<AppState> {
    let id = self.listeners.insert(Box::new(func));

    let container = self
      .listeners_tags
      .entry(event_type)
      .or_insert_with(|| Arena::new());
    let container_id = container.insert(id);

    (event_type, container_id)
  }

  pub fn remove_listener(&mut self, id: WindowEventSessionRemoveToken<AppState>) {
    let listeners = &mut self.listeners;
    self
      .listeners_tags
      .get(&id.0)
      .map(|a| a.get(id.1).map(|l_id| listeners.remove(*l_id)));
  }

  pub fn new() -> Self {
    Self {
      listeners: Arena::new(),
      listeners_tags: HashMap::new(),
    }
  }

  pub fn event(
    &mut self,
    event: &winit::event::Event<()>,
    s: &mut AppState,
    renderer: &mut AppRenderCtx,
  ) {
    let mut event_ctx = EventCtx {
      event: &event,
      state: s,
      render_ctx: renderer,
    };

    emit_listener(
      EventType::Raw,
      &mut self.listeners,
      &mut self.listeners_tags,
      &mut event_ctx,
    );

    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          emit_listener(
            EventType::Resize,
            &mut self.listeners,
            &mut self.listeners_tags,
            &mut event_ctx,
          );
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => emit_listener(
              EventType::MouseDown,
              &mut self.listeners,
              &mut self.listeners_tags,
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
              EventType::MouseWheel,
              &mut self.listeners,
              &mut self.listeners_tags,
              &mut event_ctx,
            );
          }
        }
        // WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { .. } => emit_listener(
          EventType::MouseMotion,
          &mut self.listeners,
          &mut self.listeners_tags,
          &mut event_ctx,
        ),
        _ => (),
      },
      event::Event::MainEventsCleared => emit_listener(
        EventType::EventCleared,
        &mut self.listeners,
        &mut self.listeners_tags,
        &mut event_ctx,
      ),

      _ => {}
    }
  }
}
