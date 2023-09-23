use futures::*;
use interphaser::*;
use reactive::once_forever_pending;

#[derive(Default, Clone)]
pub enum InteractState {
  #[default]
  Default,
  Pressed,
  Hovering,
}

impl InteractState {
  pub fn color(self) -> DisplayColor {
    match self {
      InteractState::Default => (0.8, 0.8, 0.8, 1.0),
      InteractState::Pressed => (0.5, 0.5, 0.5, 1.0),
      InteractState::Hovering => (0.9, 0.9, 0.9, 1.0),
    }
    .into()
  }
}

fn interactive_rect<C: View>(size: impl Into<UISize<UILength>>) -> impl View + ViewNester<C> {
  let state = InteractState::use_state();

  let on_mouse_down = state.on_event(|_, _| InteractState::Pressed);
  let on_mouse_up = state.on_event(|_, _| InteractState::Hovering);
  let on_mouse_in = state.on_event(|_, _| InteractState::Hovering);
  let on_mouse_out = state.on_event(|_, _| InteractState::Default);

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
  let clicked = checked_state.modify_by_stream_by(clicked, |_, checked| *checked = !*checked);

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

pub fn text_box(
  size: impl Into<UISize<UILength>>,
  content: impl Stream<Item = String> + Unpin + 'static,
) -> (impl View, impl Stream<Item = TextEditMessage> + Unpin) {
  let edit_text = Text::default()
    .with_layout(TextLayoutConfig::SizedBox {
      line_wrap: LineWrap::Single,
      horizon_align: TextHorizontalAlignment::Left,
      vertical_align: TextVerticalAlignment::Top,
    })
    .editable();

  let changes = edit_text.nester.events.unbound_listen();

  let clicker = ClickHandler::default();
  let click_event = clicker.events.single_listen().map(|_| {});

  let text_updates = ReactiveUpdaterGroup::default()
    .with(click_event.bind(|e: &mut EditableText, _| e.nester.focus()))
    .with(content.bind(|e: &mut EditableText, t| e.nester.set_text(t)));

  let edit_text = edit_text.react(text_updates);

  let text_box = Container::sized(size)
    .padding(RectBoundaryWidth::equal(5.))
    .wrap(edit_text)
    .nest_in(clicker);

  (text_box, changes)
}

/// input and output is normalized
pub fn slider(
  binding: impl Stream<Item = f32> + Unpin + 'static,
) -> (impl View, impl Stream<Item = f32>) {
  let slider_length = 400.;
  let slider_rail_width = 40.;
  let handle_size = 40.;

  let _binding = binding.map(|v| v.clamp(0., 1.));

  let handle = interactive_rect::<Text>((handle_size, handle_size));

  let rail = interactive_rect((slider_length, slider_rail_width));

  let view = rail.wrap(absolute_group().child(AbsChild::new(handle).with_position((0., 0.))));
  let change = once_forever_pending(0.);
  (view, change)
}
