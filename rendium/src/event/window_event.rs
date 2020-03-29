use crate::application::AppRenderCtx;
use rendiation_util::IndexContainer;
use winit::event;
use winit::event::*;
use core::any::Any;

pub struct EventCtx<'a, 'b, 'c, T> {
  pub event: winit::event::Event<()>, // todo use self event
  pub state: &'a mut T,
  pub render_ctx: &'b mut AppRenderCtx<'c>,
}

type ListenerContainer<AppState> = IndexContainer<Box<dyn FnMut(&mut EventCtx<AppState>)>>;

pub struct WindowEventSession<AppState> {
  raw_listeners: ListenerContainer<AppState>,

  events_cleared_listeners: ListenerContainer<AppState>,
  mouse_down_listeners: ListenerContainer<AppState>,
  mouse_motion_listeners: ListenerContainer<AppState>,
  mouse_wheel_listeners: ListenerContainer<AppState>,
  resize_listeners: ListenerContainer<AppState>,
}

fn emit_listener<AppState>(
  listeners: &mut ListenerContainer<AppState>,
  event: &mut EventCtx<AppState>,
) {
  for listener in listeners.iter_mut() {
    listener(event)
  }
}

impl<AppState> WindowEventSession<AppState> {
  pub fn add_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(&mut self, func: T) {
    self.raw_listeners.set_item(Box::new(func));
  }

  pub fn add_mouse_down_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.mouse_down_listeners.set_item(Box::new(func))
  }

  pub fn add_resize_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.resize_listeners.set_item(Box::new(func))
  }

  pub fn add_events_clear_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self, 
    func: T,
  ) -> usize {
    self.events_cleared_listeners.set_item(Box::new(func))
  }

  pub fn add_mouse_wheel_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.mouse_wheel_listeners.set_item(Box::new(func))
  }

  pub fn add_mouse_motion_listener<T: FnMut(&mut EventCtx<AppState>) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.mouse_motion_listeners.set_item(Box::new(func))
  }

  pub fn new() -> Self {
    Self {
      raw_listeners: IndexContainer::new(),
      events_cleared_listeners: IndexContainer::new(),
      mouse_down_listeners: IndexContainer::new(),
      mouse_motion_listeners: IndexContainer::new(),
      mouse_wheel_listeners: IndexContainer::new(),
      resize_listeners: IndexContainer::new(),
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

    emit_listener(&mut self.raw_listeners, &mut event_ctx);

    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          emit_listener(&mut self.resize_listeners, &mut event_ctx);
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => emit_listener(&mut self.mouse_down_listeners, &mut event_ctx),
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
            emit_listener(&mut self.mouse_wheel_listeners, &mut event_ctx);
          }
        }
        WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          emit_listener(&mut self.mouse_motion_listeners, &mut event_ctx);
        }
        _ => (),
      },
      event::Event::EventsCleared => {
        emit_listener(&mut self.events_cleared_listeners, &mut event_ctx);
      }

      DeviceEvent => {}
    }
  }
}
