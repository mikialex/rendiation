use interphaser::*;
use rendiation_algebra::Vec4;

pub struct ButtonState {
  pressed: bool,
  hovering: bool,
}

impl Default for ButtonState {
  fn default() -> Self {
    Self {
      pressed: false,
      hovering: false,
    }
  }
}

pub fn button<T: 'static>(
  label: impl Into<Value<String, T>>,
  on_click: impl Fn(&mut T) + 'static,
) -> impl UIComponent<T> {
  let state = ButtonState::use_state();

  let enable_pressed = state.mutation(|s| s.pressed = true);
  let disable_pressed = state.mutation(|s| s.pressed = false);
  let enable_hovering = state.mutation(|s| s.hovering = true);
  let disable_hovering = state.mutation(|s| {
    s.hovering = false;
    s.pressed = false;
  });

  Text::new(label)
    .extend(Container2::size((200., 80.).into()).update_by(move |s, _| {
      s.color = state.visit(|s| {
        if s.pressed {
          Vec4::new(0.7, 0.7, 0.7, 1.0)
        } else if s.hovering {
          Vec4::new(0.9, 0.9, 0.9, 1.0)
        } else {
          Vec4::new(0.8, 0.8, 0.8, 1.0)
        }
      })
    }))
    .extend(ClickHandler::by(on_click))
    .extend(MouseInHandler::by(enable_hovering))
    .extend(MouseOutHandler::by(disable_hovering))
    .extend(MouseDownHandler::by(enable_pressed))
    .extend(MouseUpHandler::by(disable_pressed))
}
