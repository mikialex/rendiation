struct ButtonState {
  pressed: bool,
}

struct ButtonProps {
  label: String,
}

impl Component for Button {
  type State = ButtonState;
  type Props = ButtonProps;
  fn render(state: &Self::State, props: &Self::Props) -> ComponentInstance {
    Div::new()
      .on(MouseDown, |e, s, p| s.pressed = true)
      .child(Text::new(&props.label))
  }
}
