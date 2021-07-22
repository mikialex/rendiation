use rendiation_algebra::*;

use crate::*;

#[derive(PartialEq, Clone, Default)]

pub struct ViewerUI {
  test: usize,
}

pub fn create_ui(init_size: LayoutSize) -> (ViewerUI, UI<ViewerUI>) {
  let state = ViewerUI { test: 0 };

  let com = Text::new(Value::by(|s: &ViewerUI| s.test.to_string()))
    .extend(Container::size(LayoutSize {
      width: 100.,
      height: 100.,
    }))
    .extend(ClickHandler::by(|s: &mut ViewerUI| {
      s.test += 1;
    }));

  let ui = UI::create(com, init_size);

  (state, ui)
}

pub struct Button<T> {
  label: String,
  color: Vec4<f32>,
  on_click: Box<dyn Fn(&mut T)>,
}

pub fn build<T>() -> impl Component<Button<T>> {
  Text::new(Value::by(|s: &Button<T>| s.test.to_string()))
    .extend(Container::size(LayoutSize {
      width: 100.,
      height: 100.,
    }))
    .extend(ClickHandler::by(|s: &mut Button<T>| {
      s.test += 1;
    }))
}
