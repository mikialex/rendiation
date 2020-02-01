use crate::element::*;

pub trait Component<C> {
  fn render(&self) -> ElementsTree<C>;
}

pub struct ComponentInstance<C: Component<C>> {
  state: C,
  document: ElementsTree<C>,
}

impl<C: Component<C>> ComponentInstance<C> {
  pub fn new(state: C) -> Self {
    let document = state.render();
    ComponentInstance { state, document }
  }
  pub fn event(&self, event: &Event, state: &mut C) {}
}

//
//
// user code

pub struct TestCounter {
  count: usize,
  sub_item: bool,
}

impl TestCounter {
  fn add(&mut self) {
    self.count += 1;
  }
}

impl Component<TestCounter> for TestCounter {
  fn render(&self) -> ElementsTree<Self> {
    todo!()
    // let mut div = Div::new();
    // div.listener(|_, counter: &mut Self| counter.add());
    // div
  }
}
