// use futures::Stream;
// use interphaser::*;

// #[derive(Default)]
// pub enum ButtonState {
//   #[default]
//   Normal,
//   Pressed,
//   Hovering,
// }

// impl ButtonState {
//   pub fn color(&self) -> Color {
//     match self {
//       ButtonState::Normal => (0.8, 0.8, 0.8, 1.0),
//       ButtonState::Pressed => (0.7, 0.7, 0.7, 1.0),
//       ButtonState::Hovering => (0.9, 0.9, 0.9, 1.0),
//     }
//     .into()
//   }
// }

// pub fn button<T: 'static>(
//   label: impl Stream<Item = String> + Unpin,
// ) -> (impl Component, impl Stream<Item = ()>) {
//   let state = ButtonState::use_state();

//   let on_mouse_down = state.on_event_trigger(|s| *s = ButtonState::Pressed);
//   let on_mouse_up = state.on_event_trigger(|s| *s = ButtonState::Hovering);
//   let on_mouse_in = state.on_event_trigger(|s| *s = ButtonState::Hovering);
//   let on_mouse_out = state.on_event_trigger(|s| *s = ButtonState::Normal);

//   let events = EventHandlerGroup::default()
//     .with(ClickHandler::by(on_click))
//     .with(MouseInHandler::by(on_mouse_in))
//     .with(MouseOutHandler::by(on_mouse_out))
//     .with(MouseDownHandler::by(on_mouse_down))
//     .with(MouseUpHandler::by(on_mouse_up));

//   Container::sized((200., 80.))
//     .updater(color_change.bind(Container::color_input))
//     .bind(move |s, _| s.color = state.visit(|s| s.color()))
//     .nest_over(Text::default().bind(move |s, t| s.content.set(label(t))))
//     .nest_in(events)
// }
