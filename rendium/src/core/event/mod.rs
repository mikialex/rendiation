use rendiation_algebra::Vec3;

pub mod window_event;
pub use window_event::*;

pub enum ActionType {
  Down,
  Up,
}

pub enum MouseButton {
  Left,
  Right,
  Middle,
}

pub struct MouseActionEvent {
  position: Vec3<f32>,
  action: ActionType,
  mouse_button: MouseButton,
}

pub struct KeyBoardEvent{
  key: String,
  action: ActionType,
}

pub struct Event{

}