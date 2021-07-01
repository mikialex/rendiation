use crate::ui::components::*;

use super::*;

#[derive(Default, PartialEq, Clone)]
pub struct Counter;

#[derive(Default, PartialEq, Clone)]
pub struct CounterState {
  some_large_item: Vec<Button>,
  count: usize,
}

impl Component for Counter {
  type State = CounterState;
  fn build(&self, state: &Self::State, c: &mut Composer<Self>) {
    c.children(Row.init(), |c| {
      c.child(
        Button {
          label: format!("add count{}", state.count),
        }
        .init::<Self>()
        .on(|s| s.count += 1),
      )
      .child(state.some_large_item[0].init());
    })
    .child(state.some_large_item[1].init());
  }
}

#[test]
fn ui() {
  let mut ui = UI::<Counter>::new();
  ui.render();
}
