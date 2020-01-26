use crate::renderer::WGPURenderer;
use crate::window::WindowState;
use winit::event;
use winit::event::{DeviceEvent, WindowEvent};

type ListenerContainer<AppState> = Vec<Box<dyn FnMut(&mut AppState, &mut WGPURenderer)>>;

pub struct WindowEventSession<AppState> {
  events_cleared_listeners: ListenerContainer<AppState>,
  click_listeners: ListenerContainer<AppState>,
  mouse_motion_listeners: ListenerContainer<AppState>,
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

  pub fn new() -> Self {
    Self {
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
          emit_listener(&mut self.resize_listeners, state, renderer);
          log::info!("Resizing to {:?}", size);
        }
        WindowEvent::MouseInput { button, state, .. } => {
          println!("mouse click");
          // for listener in self.click_listeners.iter_mut() {
          //   listener(MouseEvent { x: 1.0, y: 1.0 }, &mut self.app_state)
          // }
        }
        WindowEvent::CursorMoved { position, .. } => {}
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
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
