use interphaser::*;

pub fn text_box(
  placeholder: impl Into<String> + 'static + Copy,
  value: impl Into<String>,
  on_change: impl Fn(&mut String) + 'static,
) -> impl UIComponent<String> {
  If::condition(
    |t: &String| t.is_empty(),
    move |t| {
      Text::default()
        .bind(move |s, _| s.content.set(placeholder))
        .extend(Container::size((200., 80.)))
    },
  )
  // .else_condition(todo!())
}
