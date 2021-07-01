use crate::ui::{Component, Composer, Primitive};

#[derive(PartialEq, Clone)]
pub struct Button {
  pub label: String,
}

impl Default for Button {
  fn default() -> Self {
    Self {
      label: String::new(),
    }
  }
}

#[derive(Default, PartialEq)]
pub struct ButtonState {
  is_hovered: bool,
}

impl Component for Button {
  type State = ButtonState;
  fn build(&self, state: &Self::State, composer: &mut Composer<Self>) {
    composer.draw_primitive(todo!()).draw_primitive(todo!());
  }
}
