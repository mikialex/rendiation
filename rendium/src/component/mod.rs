use rendiation_util::Tree;
use crate::renderer::GUIRenderer;
use crate::element::*;
use crate::event::*;

pub trait Component<C> {
  fn render(&self) -> ComponentTree<C>;
  // fn event(&mut self);
}

pub struct ComponentInstance<C: Component<C>> {
  event_received: bool,
  element_tree: ComponentTree<C>,
  need_repaint: bool, 
}

impl<C: Component<C>> ComponentInstance<C> {
  pub fn new(state: &C) -> Self {
    let element_tree = state.render();
    Self {
      event_received: false,
      element_tree,
      need_repaint: false,
    }
  }
  pub fn event(&mut self, event: &Event, state: &mut C) {
    // forward event to element tree
    // if any element react to event, mark event_received
    // self.element_tree.root.traverse(||{

    // })
  }
  pub fn update(&mut self, state: &C) {
    if self.event_received{
      self.element_tree = state.render();
      self.need_repaint = true;
    }
  }
  pub fn paint(&mut self, renderer: &mut GUIRenderer) {
    self.need_repaint = false;
    // do render
  }
}

pub struct ComponentTree<T> {
  elements: Vec<Box<dyn Element<T>>>,
}

impl<T> ComponentTree<T> {
  fn event(&self, event: &Event, state: &mut T) {
    
  }
}


//
//
// user code

impl Component<String> for String {
  fn render(&self) -> ComponentTree<Self> {
    todo!()
  }

}


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
  fn render(&self) -> ComponentTree<Self> {
    todo!()


    // let mut div = Div::new();
    // div.listener(|_, counter: &mut Self| counter.add());
    // div
  }

}
