struct ComponentInstance {
  tree: ArenaTree<DocumentElement>,
}

enum DocumentElement {
  PrimitiveElement,
  ComponentElement,
}

trait Component {
  type Props;
  type Change;
  fn render(&self, props: &Self::Props) -> ComponentInstance;
  fn update(&mut self, change: &Self::Change) -> bool;
}

struct Button {
  pressed: bool,
}

struct ButtonProps {
  label: String,
}

impl Component for Button {
  type State = ButtonState;
  fn render() -> ComponentInstance {
    Div::new()
      .on(MouseDown, |e, s, p| todo!())
      .child(Text::new())
  }
  fn update() {}
}

struct ElementBase {
  event: EventHub,
}

impl ElementBase {
  fn on() {
    todo!()
  }
}
