use std::collections::HashSet;
use winit::event;
use winit::event::*;

pub struct WindowState {
  pub size: (f32, f32),
  pub mouse_position: (f32, f32),
  pub mouse_motion: (f32, f32),
  pub is_left_mouse_down: bool,
  pub is_right_mouse_down: bool,
  pub mouse_wheel_delta: (f32, f32),
  pub pressed_key: HashSet<VirtualKeyCode>,
}

impl WindowState {
  pub fn new(size: (f32, f32)) -> Self {
    Self {
      size,
      mouse_position: (0.0, 0.0),
      mouse_motion: (0.0, 0.0),
      is_left_mouse_down: false,
      is_right_mouse_down: false,
      mouse_wheel_delta: (0.0, 0.0),
      pressed_key: HashSet::new(),
    }
  }
  pub fn update_size(&mut self, size: &winit::dpi::PhysicalSize<u32>) {
    self.size.0 = size.width as f32;
    self.size.1 = size.height as f32;
  }

  pub fn mouse_move_to(&mut self, position: &winit::dpi::PhysicalPosition<f64>) {
    self.mouse_position.0 = position.x as f32;
    self.mouse_position.1 = position.y as f32;
  }

  pub fn mouse_motion(&mut self, motion: (f64, f64)) {
    self.mouse_motion.0 = motion.0 as f32;
    self.mouse_motion.1 = motion.1 as f32;
  }

  // pub fn attach_event<T, U: FnOnce(&mut T) -> &mut Self + 'static + Copy>(
  //   &self,
  //   events: &mut WindowEventSession<T>,
  //   lens: U,
  // ) {
  //   events.active.key_down.on(move |ctx| {
  //     lens(&mut ctx.state).pressed_key.insert(*ctx.event_data);
  //   });
  //   events.active.key_up.on(move |ctx| {
  //     lens(&mut ctx.state).pressed_key.remove(ctx.event_data);
  //   });
  //   events.active.mouse_motion.on(move |ctx| {
  //     lens(&mut ctx.state).mouse_motion(*ctx.event_data);
  //   });
  //   events.active.event_cleared.on(move |ctx| {
  //     lens(&mut ctx.state).mouse_wheel_delta = (0.0, 0.0);
  //   });

  //   // need impl piority
  //   todo!()
  // }

  pub fn event(&mut self, event: &winit::event::Event<()>) {
    match event {
      event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.update_size(&size);
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
          self.mouse_move_to(&position);
        }
        WindowEvent::KeyboardInput {
          input:
            KeyboardInput {
              virtual_keycode: Some(virtual_keycode),
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
      event::Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          self.mouse_motion(*delta);
        }
        _ => (),
      },
      event::Event::MainEventsCleared => {
        self.mouse_wheel_delta = (0.0, 0.0);
      }
      _ => {}
    }
  }
}
