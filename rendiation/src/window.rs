use winit::event;
use winit::event::*;

pub struct WindowState {
  pub size: (f32, f32),
  pub physical_size: (f32, f32),
  pub hidpi_factor: f32,
  pub mouse_position: (f32, f32),
  pub mouse_motion: (f32, f32),
  pub is_left_mouse_down: bool,
  pub is_right_mouse_down: bool,
  pub mouse_wheel_delta: (f32, f32),
}

impl WindowState {
  pub fn new(size: (f32, f32), hidpi_factor: f32) -> Self {
    Self {
      size,
      physical_size: (size.0 * hidpi_factor, size.1 * hidpi_factor),
      hidpi_factor,
      mouse_position: (0.0, 0.0),
      mouse_motion: (0.0, 0.0),
      is_left_mouse_down: false,
      is_right_mouse_down: false,
      mouse_wheel_delta: (0.0, 0.0),
    }
  }
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

  pub fn event(&mut self, event: winit::event::Event<()>) {
    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.update_size(&size);
        }
        WindowEvent::MouseInput { button, state, .. } => {
          match button {
            MouseButton::Left => {
              match state {
                ElementState::Pressed => self.is_left_mouse_down = true,
                ElementState::Released => self.is_left_mouse_down = false,
              }
            }
            MouseButton::Right => {
              match state {
                ElementState::Pressed => self.is_right_mouse_down = true,
                ElementState::Released => self.is_right_mouse_down = false,
              }
            }
            _ => {}
          }
        }
        WindowEvent::MouseWheel { delta, .. } => {
          if let MouseScrollDelta::LineDelta(x, y) = delta {
            self.mouse_wheel_delta = (x, y);
          }
        }
        WindowEvent::CursorMoved { position, .. } => {
          self.mouse_move_to(&position);
        }
        _ => (),
      },
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          self.mouse_motion(delta);
        }
        _ => (),
      },
      event::Event::EventsCleared => {
        self.mouse_wheel_delta = (0.0, 0.0);
      }
      DeviceEvent => {}
    }
  }
}
