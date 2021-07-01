use crate::ui::{Component, Composer};

#[derive(Default, PartialEq, Clone)]
pub struct FlexLayout {
  pub direction: bool,
}

impl Component for FlexLayout {
  type State = ();
  fn render(&self, state: &Self::State, composer: &mut Composer<Self>) {
    // do nothing
  }
}
