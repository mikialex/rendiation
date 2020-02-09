use crate::renderer::WGPURenderer;
use rendiation_math::Vec3;
use rendiation_util::IndexContainer;
use winit::event;
use winit::event::*;

// pub enum MouseActionType {
//   Down,
//   Up,
// }

// pub enum MouseButton {
//   Left,
//   Right,
//   Middle,
// }

// pub struct MouseActionEvent {
//   position: Vec3<f32>,
//   action: MouseActionType,
//   mouse_button: MouseButton,
// }

type ListenerContainer<AppState> = IndexContainer<Box<dyn FnMut(&mut AppState, &mut WGPURenderer)>>;

pub struct WindowEventSession<AppState> {
  events_cleared_listeners: ListenerContainer<AppState>,
  mouse_down_listeners: ListenerContainer<AppState>,
  mouse_motion_listeners: ListenerContainer<AppState>,
  mouse_wheel_listeners: ListenerContainer<AppState>,
  resize_listeners: ListenerContainer<AppState>,
}

fn emit_listener<AppState>(
  listeners: &mut ListenerContainer<AppState>,
  state: &mut AppState,
  renderer: &mut WGPURenderer,
) {
  for listener in listeners.iter_mut() {
    listener(state, renderer)
  }
}

impl<AppState> WindowEventSession<AppState> {
  pub fn add_mouse_down_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.mouse_down_listeners.set_item(Box::new(func))
  }

  pub fn add_resize_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.resize_listeners.set_item(Box::new(func))
  }

  pub fn add_events_clear_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.events_cleared_listeners.set_item(Box::new(func))
  }

  pub fn add_mouse_wheel_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.mouse_wheel_listeners.set_item(Box::new(func))
  }

  pub fn add_mouse_motion_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) -> usize {
    self.mouse_motion_listeners.set_item(Box::new(func))
  }

  pub fn new() -> Self {
    Self {
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
    renderer: &mut WGPURenderer,
  ) {
    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          emit_listener(&mut self.resize_listeners, s, renderer);
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => emit_listener(&mut self.mouse_down_listeners, s, renderer),
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
            emit_listener(&mut self.mouse_wheel_listeners, s, renderer);
          }
        }
        WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          emit_listener(&mut self.mouse_motion_listeners, s, renderer);
        }
        _ => (),
      },
      event::Event::EventsCleared => {
        emit_listener(&mut self.events_cleared_listeners, s, renderer);
      }

      DeviceEvent => {}
    }
  }
}
