use crate::ui::{Component, Composer, Primitive};

#[derive(PartialEq, Clone)]
pub struct Button {
  pub label: String,
}

#[derive(Default, PartialEq)]
pub struct ButtonState {
  is_hovered: bool,
}

impl Component for Button {
  type State = ButtonState;
  fn render(&self, state: &Self::State, composer: &mut Composer<Self>) {
    composer.draw_primitive(todo!()).draw_primitive(todo!());
  }
}
