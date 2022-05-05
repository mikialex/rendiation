use interphaser::*;

pub enum ButtonState {
  Normal,
  Pressed,
  Hovering,
}
impl Default for ButtonState {
  fn default() -> Self {
    Self::Normal
  }
}

impl ButtonState {
  pub fn color(&self) -> Color {
    match self {
      ButtonState::Normal => (0.8, 0.8, 0.8, 1.0),
      ButtonState::Pressed => (0.7, 0.7, 0.7, 1.0),
      ButtonState::Hovering => (0.9, 0.9, 0.9, 1.0),
    }
    .into()
  }
}

pub fn button<T: 'static>(
  label: impl Into<Value<String, T>>,
  on_click: impl Fn(&mut T, &mut EventHandleCtx, &()) + 'static,
) -> impl UIComponent<T> {
  let mut label = label.into();
  let state = ButtonState::use_state();

  let on_mouse_down = state.on_event_trigger(|s| *s = ButtonState::Pressed);
  let on_mouse_up = state.on_event_trigger(|s| *s = ButtonState::Hovering);
  let on_mouse_in = state.on_event_trigger(|s| *s = ButtonState::Hovering);
  let on_mouse_out = state.on_event_trigger(|s| *s = ButtonState::Normal);

  let events = EventHandlerGroup::default()
    .with(ClickHandler::by(on_click))
    .with(MouseInHandler::by(on_mouse_in))
    .with(MouseOutHandler::by(on_mouse_out))
    .with(MouseDownHandler::by(on_mouse_down))
    .with(MouseUpHandler::by(on_mouse_up));

  // let transition = TimeBasedTransition {
  //   duration: 200,
  //   ty: Transition::Linear,
  // }
  // .into_animation();

  Container::sized((200., 80.))
    .bind(move |s, _| s.color = state.visit(|s| s.color()))
    .wrap(Text::default().bind(move |s, t| s.content.set(label.eval(t))))
    .extend(events)
}
