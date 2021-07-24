use rendiation_algebra::*;

use crate::*;

#[derive(PartialEq, Clone, Default)]

pub struct ViewerUI {
  test: usize,
}

pub fn create_ui(init_size: LayoutSize) -> (ViewerUI, UI<ViewerUI>) {
  let state = ViewerUI { test: 0 };

  // let com = Text::new(Value::by(|s: &ViewerUI| s.test.to_string()))
  //   .extend(Container::size(LayoutSize {
  //     width: 100.,
  //     height: 100.,
  //   }))
  //   .extend(ClickHandler::by(|s: &mut ViewerUI| {
  //     s.test += 1;
  //   }));

  let ui = UI::create(create_ui_prototype_2(), init_size);

  (state, ui)
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
        height: 60.,
      })
      .color(Value::by(move |s: &T| state.visit(|s| s.color))),
    )
    .extend(ClickHandler::by(on_click))
    .extend(ClickHandler::by(set_pressed))
    .extend(ClickHandler::by(move |s: &mut T| set_color()))
}

pub fn create_ui_prototype_2() -> impl UIComponent<ViewerUI> {
  button(
    Value::by(|viewer: &ViewerUI| viewer.test.to_string()),
    |viewer: &mut ViewerUI| viewer.test += 1,
  )
}
