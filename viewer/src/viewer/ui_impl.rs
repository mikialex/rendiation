use interphaser::*;
use rendiation_algebra::*;

#[derive(PartialEq, Clone, Default)]

pub struct Counter {
  pub count: usize,
}

pub struct ButtonState {
  pressed: bool,
  pressed2: bool,
  color: Vec4<f32>,
}

impl Default for ButtonState {
  fn default() -> Self {
    Self {
      pressed: false,
      pressed2: false,
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
  let set_pressed = state.mutation(|s| s.pressed2 = false);

  Text::new(label)
    .extend(
      Container::size(LayoutSize {
        width: 200.,
        height: 80.,
      })
      .color(Value::by(move |s: &T| state.visit(|s| s.color))),
    )
    .extend(ClickHandler::by(on_click))
    .extend(ClickHandler::by(set_pressed))
    .extend(ClickHandler::by(move |s: &mut T| set_color()))
}
