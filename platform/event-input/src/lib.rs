#![feature(duration_millis_float)]

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use fast_hash_collection::*;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;
use winit::{
  event::*,
  keyboard::{KeyCode, PhysicalKey},
};

#[derive(Default)]
pub struct PlatformEventInput {
  pub accumulate_events: Vec<Event<()>>,
  pub previous_frame_window_state: WindowState,
  pub window_state: WindowState,
  pub state_delta: WindowStateChange,
  pub last_frame_cpu_time_in_ms: f32,
  pub current_frame_time_start: Option<Instant>,
}

impl PlatformEventInput {
  pub fn queue_event(&mut self, event: Event<()>) {
    self.accumulate_events.push(event);
  }
  pub fn begin_frame(&mut self) {
    for e in &self.accumulate_events {
      self.window_state.event(e);
    }
    self.state_delta = self.window_state.compare(&self.previous_frame_window_state);
    self.current_frame_time_start = Some(Instant::now());
  }

  pub fn end_frame(&mut self) {
    self.accumulate_events.clear();
    self.previous_frame_window_state = self.window_state.clone();
    self.window_state.reset_in_frame_states();
    self.last_frame_cpu_time_in_ms = self
      .current_frame_time_start
      .take()
      .unwrap()
      .elapsed()
      .as_millis_f32();
    if self.last_frame_cpu_time_in_ms > 100. {
      log::warn!(
        "long frame cpu time: {:2?} ms",
        self.last_frame_cpu_time_in_ms
      );
    }
  }
}

#[derive(Clone)]
pub struct WindowState {
  /// size in real pixel unit
  pub physical_size: (f32, f32),
  pub device_pixel_ratio: f32,
  pub has_any_mouse_event: bool,
  /// in physical pixel unit
  pub mouse_position: (f32, f32),
  pub left_mouse_state: ElementState,
  pub right_mouse_state: ElementState,
  pub pressed_keys: FastHashSet<KeyCode>,
}

impl WindowState {
  pub fn reset_in_frame_states(&mut self) {
    self.has_any_mouse_event = false;
  }
  pub fn compare(&self, old: &WindowState) -> WindowStateChange {
    fn compare_button_state(old: ElementState, new: ElementState) -> Option<ElementState> {
      if old == new {
        None
      } else {
        Some(new)
      }
    }

    let mut key_state_changes = FastHashMap::<KeyCode, ElementState>::default();

    for key in &self.pressed_keys {
      if !old.pressed_keys.contains(key) {
        key_state_changes.insert(*key, ElementState::Pressed);
      }
    }

    for key in &old.pressed_keys {
      if !self.pressed_keys.contains(key) {
        key_state_changes.insert(*key, ElementState::Released);
      }
    }

    WindowStateChange {
      size_change: self.physical_size != old.physical_size,
      mouse_position_change: self.mouse_position != old.mouse_position,
      left_mouse_action: compare_button_state(old.left_mouse_state, self.left_mouse_state),
      right_mouse_action: compare_button_state(old.right_mouse_state, self.right_mouse_state),
      key_state_changes,
    }
  }
  pub fn is_left_mouse_pressed(&self) -> bool {
    matches!(self.left_mouse_state, ElementState::Pressed)
  }
  pub fn is_left_mouse_released(&self) -> bool {
    matches!(self.left_mouse_state, ElementState::Released)
  }
  pub fn is_right_mouse_pressed(&self) -> bool {
    matches!(self.right_mouse_state, ElementState::Pressed)
  }
  pub fn is_right_mouse_released(&self) -> bool {
    matches!(self.right_mouse_state, ElementState::Released)
  }
}

#[derive(Default)]
pub struct WindowStateChange {
  pub size_change: bool,
  pub mouse_position_change: bool,
  pub left_mouse_action: Option<ElementState>,
  pub right_mouse_action: Option<ElementState>,
  /// if the ElementState::Release, the key will not inside the pressed_keys
  pub key_state_changes: FastHashMap<KeyCode, ElementState>,
}

impl WindowStateChange {
  pub fn is_left_mouse_pressing(&self) -> bool {
    matches!(self.left_mouse_action, Some(ElementState::Pressed))
  }
  pub fn is_left_mouse_releasing(&self) -> bool {
    matches!(self.left_mouse_action, Some(ElementState::Released))
  }
  pub fn is_right_mouse_pressing(&self) -> bool {
    matches!(self.right_mouse_action, Some(ElementState::Pressed))
  }
  pub fn is_right_mouse_releasing(&self) -> bool {
    matches!(self.right_mouse_action, Some(ElementState::Released))
  }
}

impl WindowState {
  #[allow(clippy::single_match)]
  pub fn event(&mut self, event: &winit::event::Event<()>) {
    match event {
      winit::event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.physical_size.0 = size.width as f32;
          self.physical_size.1 = size.height as f32;
        }
        WindowEvent::MouseInput { button, state, .. } => {
          match button {
            MouseButton::Left => self.left_mouse_state = *state,
            MouseButton::Right => self.right_mouse_state = *state,
            _ => {}
          }
          self.has_any_mouse_event = true
        }
        WindowEvent::KeyboardInput { event, .. } => {
          if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
            match event.state {
              ElementState::Pressed => {
                self.pressed_keys.insert(key);
              }
              ElementState::Released => {
                self.pressed_keys.remove(&key);
              }
            }
          }
        }
        WindowEvent::CursorMoved { position, .. } => {
          self.mouse_position.0 = position.x as f32;
          self.mouse_position.1 = position.y as f32;
          self.has_any_mouse_event = true
        }
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
          self.device_pixel_ratio = *scale_factor as f32;
        }
        _ => (),
      },
      _ => {}
    }
  }
}

impl Default for WindowState {
  fn default() -> Self {
    Self {
      physical_size: (0.0, 0.0),
      device_pixel_ratio: 1.0,
      mouse_position: (0.0, 0.0),
      left_mouse_state: ElementState::Released,
      right_mouse_state: ElementState::Released,
      has_any_mouse_event: false,
      pressed_keys: Default::default(),
    }
  }
}

// pub struct CanvasWindowPositionInfo {
//   /// in window coordinates
//   pub absolute_position: Vec2<f32>,
//   pub size: Vec2<f32>,
// }

// impl CanvasWindowPositionInfo {
//   pub fn full_window(window_size: (f32, f32)) -> Self {
//     Self {
//       absolute_position: Vec2::new(0., 0.),
//       size: Vec2::new(window_size.0, window_size.1),
//     }
//   }
// }

// impl CanvasWindowPositionInfo {
//   pub fn compute_normalized_position_in_canvas_coordinate(
//     &self,
//     states: &WindowState,
//   ) -> (f32, f32) {
//     let canvas_x = states.mouse_position.0 - self.absolute_position.x;
//     let canvas_y = states.mouse_position.1 - self.absolute_position.y;

//     (
//       canvas_x / self.size.x * 2. - 1.,
//       -(canvas_y / self.size.y * 2. - 1.),
//     )
//   }
// }

pub fn window_event(event: &Event<()>) -> Option<&WindowEvent> {
  match event {
    Event::WindowEvent { event, .. } => Some(event),
    _ => None,
  }
}

pub fn mouse(event: &Event<()>) -> Option<(MouseButton, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::MouseInput { state, button, .. } => Some((*button, *state)),
    _ => None,
  })
}

pub fn keyboard(event: &Event<()>) -> Option<(Option<KeyCode>, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::KeyboardInput {
      event: KeyEvent {
        physical_key,
        state,
        ..
      },
      ..
    } => Some((
      match physical_key {
        PhysicalKey::Code(code) => Some(*code),
        _ => None,
      },
      *state,
    )),
    _ => None,
  })
}

pub fn mouse_move(event: &Event<()>) -> Option<winit::dpi::PhysicalPosition<f64>> {
  window_event(event).and_then(|e| match e {
    WindowEvent::CursorMoved { position, .. } => Some(*position),
    _ => None,
  })
}
