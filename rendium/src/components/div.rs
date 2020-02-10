use crate::component::ComponentTree;
use crate::component::Component;

pub struct Div {}

impl Component<Div> for Div {
  fn render(&self) -> ComponentTree<Div> {
    unimplemented!()
  }
}
