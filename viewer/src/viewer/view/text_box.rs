pub fn text_box<T>(
  place_holder: impl Into<Value<String, T>>,
  value: impl Into<Value<String, T>>,
  on_change: impl Fn(&mut T) + 'static,
) -> impl UIComponent<T> {
  If::condition(
    |t| value.eval(t) == "",
    |t| Text::default().updater(move |s, t| s.content.set(place_holder.eval(t))),
  )
}
