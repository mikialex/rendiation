use futures::*;
use interphaser::*;
use reactive::EventSource;

#[derive(Default)]
pub enum ButtonState {
  #[default]
  Normal,
  Pressed,
  Hovering,
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

// pub fn button(label: String) -> (impl Component, impl Stream<Item = ()>) {
//   let state = ButtonState::use_state();

//   // let on_mouse_down = state.on_event_trigger(|s| *s = ButtonState::Pressed);
//   // let on_mouse_up = state.on_event_trigger(|s| *s = ButtonState::Hovering);
//   // let on_mouse_in = state.on_event_trigger(|s| *s = ButtonState::Hovering);
//   // let on_mouse_out = state.on_event_trigger(|s| *s = ButtonState::Normal);

//   // let events = EventHandlerGroup::default()
//   //   .with(ClickHandler::by(on_click))
//   //   .with(MouseInHandler::by(on_mouse_in))
//   //   .with(MouseOutHandler::by(on_mouse_out))
//   //   .with(MouseDownHandler::by(on_mouse_down))
//   //   .with(MouseUpHandler::by(on_mouse_up));

//   let color = EventSource::<Color>::default();
//   let color_change = color.single_listen();

//   let clicker = ClickHandler::default();
//   let click_event = clicker.events.single_listen().map(|_| {});

//   let view = Container::sized((200., 80.))
//     .nest_in(color_change.bind(Container::set_color))
//     .wrap(Text::new(label))
//     .nest_in(clicker);

//   (view, click_event)
// }
