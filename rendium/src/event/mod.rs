use rendiation_math::Vec3;

pub enum MouseActionType {
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
  action: MouseActionType,
  mouse_button: MouseButton,
}

pub struct Event{
    
}