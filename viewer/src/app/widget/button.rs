use futures::*;
use interphaser::*;

#[derive(Default, Clone)]
pub enum ButtonState {
  #[default]
  Normal,
  Pressed,
  Hovering,
}

impl ButtonState {
  pub fn color(self) -> Color {
    match self {
      ButtonState::Normal => (0.8, 0.8, 0.8, 1.0),
      ButtonState::Pressed => (0.7, 0.7, 0.7, 1.0),
      ButtonState::Hovering => (0.9, 0.9, 0.9, 1.0),
    }
    .into()
  }
}

pub fn button(label: String) -> (impl View, impl Stream<Item = ()>) {
  let state = ButtonState::use_state();

  let on_mouse_down = state.on_event(|_, _| ButtonState::Pressed);
  let on_mouse_up = state.on_event(|_, _| ButtonState::Hovering);
  let on_mouse_in = state.on_event(|_, _| ButtonState::Hovering);
  let on_mouse_out = state.on_event(|_, _| ButtonState::Normal);

  let events = EventHandlerGroup::default()
    .with(MouseInHandler::on(on_mouse_in))
    .with(MouseOutHandler::on(on_mouse_out))
    .with(MouseDownHandler::on(on_mouse_down))
    .with(MouseUpHandler::on(on_mouse_up));

  let color = state.single_listen().map(ButtonState::color);

  let (clicker, clicked) = ClickHandler::any_triggered();

  let view = Container::sized((200., 80.))
    .react(color.bind(Container::set_color))
    .into_state_holder()
    .hold_state(state)
    .wrap(Text::new(label))
    .nest_in(events)
    .nest_in(clicker);

  (view, clicked)
}
