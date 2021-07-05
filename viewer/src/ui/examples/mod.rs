use crate::ui::components::*;

use super::*;

#[derive(Default, PartialEq, Clone, Debug)]
pub struct Counter {
  n: usize,
}

#[derive(Default, PartialEq, Clone)]
pub struct CounterState {
  some_large_item: Vec<Button>,
  count: usize,
  a: bool,
}

impl Component for Counter {
  type State = CounterState;
  fn build(model: &mut Model<Self>, c: &mut Composer<Self>) {
    c.children(Container::default().init(), |c| {
      let count = model.view(|s| s.state.count);
      // let count2 = model.view(|s| s.state.count);
      // let count3 = model.compute(|m| m.get(count2) + 324);

      c.child(
        Button {
          label: format!("add count{}", count),
        }
        .init::<Self>()
        .on(|s| s.state.count += 1),
      )
      .child(model.view(|s| s.state.some_large_item[0].clone()).init());
    })
    .children(Container::default().init(), |c| {
      //
    })
    .child(
      model
        .view(|s| s.state.some_large_item[2].clone())
        .init::<Self>()
        .on(|s| println!("{:?}", s.props)),
    );

    let should = model.view(|s| s.state.a && s.props.n > 0);
    if *should {
      c.children(Container::default().init(), |c| {
        //
      });
    }
  }
}

#[test]
fn ui() {
  let mut ui = UI::<Counter>::new();
  ui.render();
}
