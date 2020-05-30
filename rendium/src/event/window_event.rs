use crate::application::AppRenderCtx;
// use core::any::Any;
use generational_arena::*;
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
}

pub struct WindowEventSession<AppState> {
  raw_listeners: Arena<(Option<(EventType, Index)>, ListenerStorage<AppState>)>,
  fixed_listeners: HashMap<EventType, Arena<Index>>,
}

fn emit_listener<AppState>(
  raw_listeners: &mut Arena<(Option<(EventType, Index)>, ListenerStorage<AppState>)>,
  listeners: Option<&mut Arena<Index>>,
  event: &mut EventCtx<AppState>,
) {
  if let Some(listeners) = listeners {
    listeners.iter_mut().for_each(|(_, f)| {
      let (_, function) = raw_listeners.get_mut(*f).unwrap();
      function(event)
    })
  }
}

impl<AppState> WindowEventSession<AppState> {
  pub fn add_listener_raw<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    func: T,
  ) -> Index {
    self.raw_listeners.insert((None, Box::new(func)))
  }

  pub fn add_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    event_type: EventType,
    func: T,
  ) -> Index {
    let id = self.raw_listeners.insert((
      Some((event_type, Index::from_raw_parts(0, 0))),
      Box::new(func),
    ));

    let container = self
      .fixed_listeners
      .entry(event_type)
      .or_insert_with(|| Arena::new());
    let raw_index = container.insert(id);

    self.raw_listeners.get(id).unwrap().0.unwrap().1 = raw_index;
    id
  }

  pub fn remove_listener(&mut self, id: Index) {
    if let Some((Some((event_type, index)), _)) = self.raw_listeners.get(id) {
      let container = self
        .fixed_listeners
        .entry(*event_type)
        .or_insert_with(|| Arena::new());
      let raw_index = *container.get(*index).unwrap();
      container.remove(*index);
      self.raw_listeners.remove(raw_index);
    } else {
      panic!("corrupt listener")
    }
  }

  pub fn new() -> Self {
    Self {
      raw_listeners: Arena::new(),
      fixed_listeners: HashMap::new(),
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

    self.raw_listeners.iter_mut().for_each(|(_, (t, f))| {
      if t.is_none() {
        f(&mut event_ctx);
      }
    });

    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          emit_listener(
            &mut self.raw_listeners,
            self.fixed_listeners.get_mut(&EventType::Resize),
            &mut event_ctx,
          );
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => emit_listener(
              &mut self.raw_listeners,
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
              &mut self.raw_listeners,
              self.fixed_listeners.get_mut(&EventType::MouseWheel),
              &mut event_ctx,
            );
          }
        }
        // WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { .. } => emit_listener(
          &mut self.raw_listeners,
          self.fixed_listeners.get_mut(&EventType::MouseMotion),
          &mut event_ctx,
        ),
        _ => (),
      },
      event::Event::MainEventsCleared => emit_listener(
        &mut self.raw_listeners,
        self.fixed_listeners.get_mut(&EventType::EventCleared),
        &mut event_ctx,
      ),

      _ => {}
    }
  }
}
