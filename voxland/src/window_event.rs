use crate::application::AppRenderCtx;
// use core::any::Any;
use arena::*;
use winit::event;
use winit::{dpi::PhysicalSize, event::*};

pub struct EventCtx<'a, 'b, 'c, T, U> {
  pub event_raw: &'a winit::event::Event<'b, ()>,
  pub event_data: &'a U,
  pub state: &'a mut T,
  pub render_ctx: &'a mut AppRenderCtx<'c>,
  pub event_update_ctx: &'a mut WindowEventSessionUpdateCtx<T>,
}

impl<'a, 'b, 'c, T, U> EventCtx<'a, 'b, 'c, T, U> {
  pub fn use_data<V>(self, event_data: &'a V) -> EventCtx<'a, 'b, 'c, T, V> {
    EventCtx {
      event_raw: self.event_raw,
      event_data: event_data,
      state: self.state,
      render_ctx: self.render_ctx,
      event_update_ctx: self.event_update_ctx,
    }
  }
}

pub struct EventUpdateCtx<'a, T> {
  pub state: &'a mut T,
  pub active_event: &'a mut WindowEventSessionData<T>,
}

type ListenerStorage<T, U> = Box<dyn FnMut(&mut EventCtx<T, U>)>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum EventType {
  EventCleared,
  MouseDown,
  MouseMotion,
  MouseWheel,
  KeyDown,
  KeyUp,
  KeyInput,
  Resize,
  Raw,
}

pub struct AnyEvent {}

pub struct WindowEventSessionRemoveToken<T>(EventType, Handle<ListenerStorage<T, AnyEvent>>);

impl<T> WindowEventSessionRemoveToken<T> {
  pub fn inner<U>(self) -> Handle<ListenerStorage<T, U>> {
    unsafe { self.1.cast_type() }
  }
}

impl EventType {
  pub fn wrap_as_token<T, U>(
    self,
    handle: Handle<ListenerStorage<T, U>>,
  ) -> WindowEventSessionRemoveToken<T> {
    unsafe { WindowEventSessionRemoveToken(self, handle.cast_type()) }
  }
}

pub struct WindowEventSessionUpdateCtx<T> {
  mutator: Vec<Box<dyn FnOnce(&mut EventUpdateCtx<T>)>>,
}

impl<T> WindowEventSessionUpdateCtx<T> {
  pub fn mutate<U: FnOnce(&mut EventUpdateCtx<T>) + 'static>(&mut self, f: U) {
    self.mutator.push(Box::new(f));
  }
}

impl<T> WindowEventSessionUpdateCtx<T> {
  pub fn new() -> Self {
    Self {
      mutator: Vec::new(),
    }
  }
}

pub struct TypedEventSessionData<T, U> {
  listeners: Arena<ListenerStorage<T, U>>,
}

impl<T, U> TypedEventSessionData<T, U> {
  pub fn new() -> Self {
    Self {
      listeners: Arena::new(),
    }
  }

  pub fn on<F: FnMut(&mut EventCtx<T, U>) + 'static>(
    &mut self,
    func: F,
  ) -> Handle<ListenerStorage<T, U>> {
    self.listeners.insert(Box::new(func))
  }

  pub fn off(&mut self, id: Handle<ListenerStorage<T, U>>) {
    self.listeners.remove(id);
  }

  pub fn emit(&mut self, event: &mut EventCtx<T, U>) {
    self.listeners.iter_mut().for_each(|(_, l)| {
      l(event);
    })
  }
}

pub struct WindowEventSessionData<T> {
  pub raw: TypedEventSessionData<T, ()>,
  pub resize: TypedEventSessionData<T, PhysicalSize<u32>>,
  pub event_cleared: TypedEventSessionData<T, ()>,
  pub key_input: TypedEventSessionData<T, KeyboardInput>,
  pub key_down: TypedEventSessionData<T, VirtualKeyCode>,
  pub key_up: TypedEventSessionData<T, VirtualKeyCode>,
  pub mouse_down: TypedEventSessionData<T, MouseButton>,
  pub mouse_motion: TypedEventSessionData<T, (f64, f64)>,
  pub mouse_wheel: TypedEventSessionData<T, ()>,
}

impl<T> WindowEventSessionData<T> {
  pub fn new() -> Self {
    Self {
      raw: TypedEventSessionData::new(),
      resize: TypedEventSessionData::new(),
      event_cleared: TypedEventSessionData::new(),
      key_input: TypedEventSessionData::new(),
      key_down: TypedEventSessionData::new(),
      key_up: TypedEventSessionData::new(),
      mouse_down: TypedEventSessionData::new(),
      mouse_motion: TypedEventSessionData::new(),
      mouse_wheel: TypedEventSessionData::new(),
    }
  }

  pub fn remove_by_token(&mut self, token: WindowEventSessionRemoveToken<T>) {
    use EventType::*;
    match token.0 {
      EventCleared => self.event_cleared.off(token.inner()),
      MouseDown => self.mouse_down.off(token.inner()),
      MouseMotion => self.mouse_motion.off(token.inner()),
      MouseWheel => self.mouse_wheel.off(token.inner()),
      Resize => self.resize.off(token.inner()),
      KeyInput => self.key_input.off(token.inner()),
      KeyDown => self.key_down.off(token.inner()),
      KeyUp => self.key_up.off(token.inner()),
      Raw => self.raw.off(token.inner()),
    }
  }
}

pub struct WindowEventSession<T> {
  pub active: WindowEventSessionData<T>,
  update_ctx: WindowEventSessionUpdateCtx<T>,
}

impl<T> WindowEventSession<T> {
  pub fn new() -> Self {
    Self {
      active: WindowEventSessionData::new(),
      update_ctx: WindowEventSessionUpdateCtx::new(),
    }
  }

  pub fn event(&mut self, event: &winit::event::Event<()>, s: &mut T, renderer: &mut AppRenderCtx) {
    let mut event_ctx = EventCtx {
      event_raw: &event,
      event_data: &(),
      state: s,
      render_ctx: renderer,
      event_update_ctx: &mut self.update_ctx,
    };

    let active = &mut self.active;
    active.raw.emit(&mut event_ctx);

    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          active.resize.emit(&mut event_ctx.use_data(size));
          //   log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => active.mouse_down.emit(&mut event_ctx.use_data(button)),
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
            active.mouse_wheel.emit(&mut event_ctx);
          }
        }
        WindowEvent::KeyboardInput { input, .. } => {
          let mut event_ctx = event_ctx.use_data(input);
          active.key_input.emit(&mut event_ctx);
          if let KeyboardInput {
            virtual_keycode: Some(virtual_keycode),
            state,
            ..
          } = input
          {
            if *state == ElementState::Pressed {
              active
                .key_down
                .emit(&mut event_ctx.use_data(virtual_keycode));
            } else {
              active.key_up.emit(&mut event_ctx.use_data(virtual_keycode));
            }
          }
        }
        // WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          active.mouse_motion.emit(&mut event_ctx.use_data(delta))
        }
        _ => (),
      },
      event::Event::MainEventsCleared => active.event_cleared.emit(&mut event_ctx),
      _ => {}
    }

    // update event self
    let mut event_ctx = EventUpdateCtx {
      state: s,
      active_event: &mut self.active,
    };
    self.update_ctx.mutator.drain(..).for_each(|m| {
      m(&mut event_ctx);
    })
  }
}
