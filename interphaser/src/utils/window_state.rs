use winit::{event::*, keyboard::KeyCode};

use crate::*;

pub struct WindowState {
  pub size: UISize,
  pub mouse_position: UIPosition,
  pub mouse_motion: (f32, f32),
  pub is_left_mouse_down: bool,
  pub is_right_mouse_down: bool,
  pub mouse_wheel_delta: (f32, f32),
  pub pressed_key: FastHashSet<KeyCode>,
  pub device_pixel_ratio: f32,
}

impl WindowState {
  pub fn new(initial_size: UISize, device_pixel_ratio: f32) -> Self {
    Self {
      size: initial_size,
      mouse_position: Default::default(),
      mouse_motion: (0.0, 0.0),
      is_left_mouse_down: false,
      is_right_mouse_down: false,
      mouse_wheel_delta: (0.0, 0.0),
      pressed_key: Default::default(),
      device_pixel_ratio,
    }
  }
  fn update_size(&mut self, size: &winit::dpi::PhysicalSize<u32>) {
    self.size.width = size.width as f32;
    self.size.height = size.height as f32;
  }

  fn mouse_move_to(&mut self, position: &winit::dpi::PhysicalPosition<f64>) {
    self.mouse_position.x = position.x as f32;
    self.mouse_position.y = position.y as f32;
  }

  fn mouse_motion(&mut self, motion: (f64, f64)) {
    self.mouse_motion.0 = motion.0 as f32;
    self.mouse_motion.1 = motion.1 as f32;
  }

  pub fn event(&mut self, event: &winit::event::Event<()>) {
    match event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.update_size(size);
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
          self.device_pixel_ratio = *scale_factor as f32;
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => self.is_left_mouse_down = true,
            ElementState::Released => self.is_left_mouse_down = false,
          },
          MouseButton::Right => match state {
            ElementState::Pressed => self.is_right_mouse_down = true,
            ElementState::Released => self.is_right_mouse_down = false,
          },
          _ => {}
        },
        WindowEvent::MouseWheel { delta, .. } => {
          if let MouseScrollDelta::LineDelta(x, y) = delta {
            self.mouse_wheel_delta = (*x, *y);
          }
        }
        WindowEvent::CursorMoved { position, .. } => {
          self.mouse_move_to(position);
        }
        WindowEvent::KeyboardInput {
          event:
            KeyEvent {
              physical_key: winit::keyboard::PhysicalKey::Code(virtual_keycode),
              state,
              ..
            },
          ..
        } => {
          let pressed = *state == ElementState::Pressed;
          if pressed {
            self.pressed_key.insert(*virtual_keycode);
          } else {
            self.pressed_key.remove(virtual_keycode);
          }
        }
        _ => (),
      },
      Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          self.mouse_motion(*delta);
        }
        _ => (),
      },
      Event::MainEventsCleared => {
        self.mouse_wheel_delta = (0.0, 0.0);
      }
      _ => {}
    }
  }
}
