trait Component {
  type State: Default;
  type Props;
  fn render(state: &Self::State, props: &Self::Props) -> ArenaTree<DocumentElement>;
}

struct ComponentInstance {
  tree: ArenaTree<DocumentElement>,
}

impl ComponentInstance {
  pub fn create() -> Self {}
}

enum DocumentElement {
  PrimitiveElement,
  ComponentElement(Box<ComponentInstance>),
}

pub struct Document {
  tree: ArenaTree<Element>,
  active_element: Option<ElementHandle>,
  hovering_element: Option<ElementHandle>,
  event: EventHub,
}
