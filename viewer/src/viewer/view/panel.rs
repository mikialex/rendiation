use interphaser::*;



pub fn panel<T: 'static>(title: impl Lens<T, String> + 'static) -> impl UIComponent<T> {
  build_title().lens(title)
}

fn build_title() -> impl UIComponent<String> {
  Text::default()
    .bind(|s, t| s.content.set(t))
    .extend(Container::size((500., 40.)))
}
