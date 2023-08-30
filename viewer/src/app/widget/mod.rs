use futures::*;
use interphaser::*;

#[derive(Default, Clone)]
pub enum InteractState {
  #[default]
  Normal,
  Pressed,
  Hovering,
}

impl InteractState {
  pub fn color(self) -> Color {
    match self {
      InteractState::Normal => (0.8, 0.8, 0.8, 1.0),
      InteractState::Pressed => (0.7, 0.7, 0.7, 1.0),
      InteractState::Hovering => (0.9, 0.9, 0.9, 1.0),
    }
    .into()
  }
}

fn interactive_rect(size: impl Into<UISize<UILength>>) -> impl View {
  let state = InteractState::use_state();

  let on_mouse_down = state.on_event(|_, _| InteractState::Pressed);
  let on_mouse_up = state.on_event(|_, _| InteractState::Hovering);
  let on_mouse_in = state.on_event(|_, _| InteractState::Hovering);
  let on_mouse_out = state.on_event(|_, _| InteractState::Normal);

  let events = EventHandlerGroup::default()
    .with(MouseInHandler::on(on_mouse_in))
    .with(MouseOutHandler::on(on_mouse_out))
    .with(MouseDownHandler::on(on_mouse_down))
    .with(MouseUpHandler::on(on_mouse_up));

  let color = state.single_listen().map(InteractState::color);

  Container::sized(size)
    .react(color.bind(Container::set_color))
    .into_any_holder()
    .hold_state(state)
    .nest_in(events)
}

pub fn button(label: String) -> (impl View, impl Stream<Item = ()>) {
  let (clicker, clicked) = ClickHandler::any_triggered();

  let view = interactive_rect((200., 80.))
    .nest_in(clicker)
    .wrap(Text::new(label));

  (view, clicked)
}

pub fn checkbox(
  binding: impl Stream<Item = bool> + Unpin + 'static,
) -> (impl View, impl Stream<Item = bool>) {
  let (clicker, clicked) = ClickHandler::any_triggered();
  let checked_state = bool::use_state();

  let color_map = |checked| {
    if checked {
      (0., 0., 0., 1.0)
    } else {
      (0., 0., 0., 0.0)
    }
    .into()
  };
  let color = checked_state.single_listen().map(color_map);
  let stream_out = checked_state.single_listen();

  let binding = checked_state.modify_by_stream(binding).map(|_| ());
  let clicked = checked_state.modify_by_stream_by(clicked, |_, checked| !checked);

  let check_flag = Container::sized((40., 40.))
    .react(color.bind(Container::set_color))
    .into_any_holder()
    .hold_stream(clicked)
    .hold_stream(binding)
    .hold_state(checked_state);

  let view = interactive_rect((50., 50.))
    .nest_in(clicker)
    .wrap(check_flag);

  (view, stream_out)
}
