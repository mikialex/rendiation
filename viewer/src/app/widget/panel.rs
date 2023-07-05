// use interphaser::*;

// pub fn panel<T: 'static>(
//   title: impl Lens<T, String> + 'static,
//   panel_body: impl UIComponent<T> + 'static,
// ) -> impl UIComponent<T> {
//   let title = build_title().lens(title);

//   flex_group()
//     .child(Child::flex(title, 1.))
//     .child(Child::flex(panel_body, 1.))
//     .extend(Flex::column())
//     .extend(Container::sized((500., 600.)))
// }

// fn build_title() -> impl UIComponent<String> {
//   Text::default()
//     .bind(|s, t| s.content.set(t))
//     .extend(Container::sized((500., 40.)))
// }
