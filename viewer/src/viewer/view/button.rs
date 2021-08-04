use interphaser::*;
use rendiation_algebra::Vec4;

pub struct ButtonState {
  pressed: bool,
  hovering: bool,
  color: Vec4<f32>,
}

impl Default for ButtonState {
  fn default() -> Self {
    Self {
      pressed: false,
      hovering: false,
      color: Vec4::new(1.0, 0.0, 0.0, 1.0),
    }
  }
}

pub fn button<T: 'static>(
  label: impl Into<Value<String, T>>,
  on_click: impl Fn(&mut T) + 'static,
) -> impl UIComponent<T> {
  let state = ButtonState::use_state();
  let set_color = state.mutator(|s| s.color.y += 0.1);
  let set_pressed = state.mutation(|s| s.pressed = false);

  Text::new(label)
    .extend(
      Container::size((200., 80.).into()).color(Value::by(move |s: &T| state.visit(|s| s.color))),
    )
    .extend(ClickHandler::by(on_click))
    .extend(MouseDownHandler::by(set_pressed))
    .extend(MouseDownHandler::by(move |s: &mut T| set_color()))
}
