use crate::event::MouseEvent;
use crate::renderer::WGPURenderer;
use winit::event;
use winit::event::{DeviceEvent, WindowEvent};

pub struct WindowState {
  size: (f32, f32),
  physical_size: (f32, f32),
  hidpi_factor: f32,
  mouse_position: (f32, f32),
  mouse_motion: (f32, f32),
}

impl WindowState {
  pub fn update_size(&mut self, size: &winit::dpi::LogicalSize) {
    self.size.0 = size.width as f32;
    self.size.1 = size.height as f32;
    let physical = size.to_physical(self.hidpi_factor as f64);
    self.physical_size.0 = physical.width as f32;
    self.physical_size.1 = physical.height as f32;
  }

  pub fn mouse_move_to(&mut self, position: &winit::dpi::LogicalPosition) {
    self.mouse_position.0 = position.x as f32;
    self.mouse_position.1 = position.y as f32;
  }

  pub fn mouse_motion(&mut self, motion: (f64, f64)) {
    self.mouse_motion.0 = motion.0 as f32;
    self.mouse_motion.1 = motion.1 as f32;
  }
}

type ListenerContainer<AppState> = Vec<Box<dyn FnMut(&mut AppState, &mut WGPURenderer)>>;

pub struct Window<AppState> {
  pub window_state: WindowState,

  events_cleared_listeners: ListenerContainer<AppState>,
  click_listeners: ListenerContainer<AppState>,
  mouse_motion_listeners: ListenerContainer<AppState>,
  resize_listeners: ListenerContainer<AppState>,
}

// pub struct WindowEventSession<T: FnMut> {
//   events_cleared_listeners: ListenerContainer<AppState>,
//   click_listeners: ListenerContainer<AppState>,
//   resize_listeners: ListenerContainer<AppState>,
// }

fn emit_listener<AppState>(
  listeners: &mut ListenerContainer<AppState>,
  state: &mut AppState,
  renderer: &mut WGPURenderer,
) {
  for listener in listeners.iter_mut() {
    listener(state, renderer)
  }
}

impl<AppState> Window<AppState> {
  pub fn add_mouse_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) {
    self.click_listeners.push(Box::new(func));
  }

  pub fn add_resize_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) {
    self.resize_listeners.push(Box::new(func));
  }

  pub fn add_events_clear_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) {
    self.events_cleared_listeners.push(Box::new(func));
  }

  pub fn add_mouse_motion_listener<T: FnMut(&mut AppState, &mut WGPURenderer) + 'static>(
    &mut self,
    func: T,
  ) {
    self.mouse_motion_listeners.push(Box::new(func));
  }

  pub fn new(size: (f32, f32), hidpi_factor: f32) -> Self {
    Window {
      window_state: WindowState {
        size,
        physical_size: (size.0 * hidpi_factor, size.1 * hidpi_factor),
        hidpi_factor,
        mouse_position: (0.0, 0.0),
        mouse_motion: (0.0, 0.0),
      },
      events_cleared_listeners: Vec::new(),
      click_listeners: Vec::new(),
      mouse_motion_listeners: Vec::new(),
      resize_listeners: Vec::new(),
    }
  }

  pub fn event(
    &mut self,
    event: winit::event::Event<()>,
    state: &mut AppState,
    renderer: &mut WGPURenderer,
  ) {
    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.window_state.update_size(&size);
          emit_listener(&mut self.resize_listeners, state, renderer);
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => {
          println!("mouse click");
          // for listener in self.click_listeners.iter_mut() {
          //   listener(MouseEvent { x: 1.0, y: 1.0 }, &mut self.app_state)
          // }
        }
        WindowEvent::CursorMoved { position, .. } => {
          self.window_state.mouse_move_to(&position);
        }
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          self.window_state.mouse_motion(delta);
          emit_listener(&mut self.mouse_motion_listeners, state, renderer);
        }
        _ => (),
      },
      event::Event::EventsCleared => {
        emit_listener(&mut self.events_cleared_listeners, state, renderer);
      }

      DeviceEvent => {}
    }
  }
}
