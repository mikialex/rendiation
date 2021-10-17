use interphaser::*;

pub fn text_box(
  place_holder: impl Into<String> + 'static,
  value: impl Into<String>,
  on_change: impl Fn(&mut String) + 'static,
) -> impl UIComponent<String> {
  If::condition(
    |t: &String| t.is_empty(),
    |t| {
      Text::default()
        // .bind(move |s, t| s.content.set(place_holder))
        .extend(Container::size((200., 80.)))
    },
  )
}
