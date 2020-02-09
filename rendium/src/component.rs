use crate::renderer::GUIRenderer;
use crate::element::*;

pub trait Component<C> {
  fn render(&self) -> ElementsTree<C>;
  fn event(&mut self);
}


pub struct UpdateCtx {
  
}

pub struct ComponentInstance<C: Component<C>> {
  state: C,
  event_received: bool,
  element_tree: ElementsTree<C>,
  need_repaint: bool, 
}

impl<C: Component<C>> ComponentInstance<C> {
  pub fn new(state: C) -> Self {
    let element_tree = state.render();
    Self {
      state,
      event_received: false,
      element_tree,
      need_repaint: false,
    }
  }
  pub fn event(&mut self, event: &Event, state: &mut C) {
    // forward event to element tree
    // if any element react to event, mark event_received
    let update_ctx = UpdateCtx{};
    // self.element_tree.root.traverse(||{

    // })
  }
  pub fn update(&mut self) {
    if self.event_received{
      self.element_tree = self.state.render();
      self.need_repaint = true;
    }
  }
  pub fn paint(&mut self, renderer: &mut GUIRenderer) {
    self.need_repaint = false;
    // do render
  }
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

  fn event(&mut self){}
}
