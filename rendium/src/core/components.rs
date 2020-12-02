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

pub struct Document {
  component_instance: ComponentInstance,
  element_tree: ArenaTree<Element>,
  active_element: Option<ElementHandle>,
  hovering_element: Option<ElementHandle>,
  event: EventHub,
}
