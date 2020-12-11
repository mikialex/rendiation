trait Component {
  type State: Default;
  type Props;
  fn render(state: &Self::State, props: &Self::Props) -> ArenaTree<DocumentElement>;
}

struct ComponentInstance {
  root_element: ElementHandle,
  tree: ArenaTree<DocumentElement>,
}

impl ComponentInstance {
  pub fn create() -> Self {}
}

enum DocumentElement {
  PrimitiveElement,
  ComponentElement(Box<ComponentInstance>),
}
