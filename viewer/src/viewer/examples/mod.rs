use interphaser::*;

pub struct Todo {
  pub items: Vec<TodoItem>,
}

#[derive(Clone, PartialEq)]
pub struct TodoItem {
  pub name: String,
}

pub fn build_todo() -> impl UIComponent<Todo> {
  For::by(|item: &TodoItem, i| Child::Flex {
    widget: Box::new(
      Text::default()
        .updater(move |s, t: &TodoItem| s.content.set(t.name.clone()))
        .extend(Container::size((500., 100.))),
    ),
    result: Default::default(),
    position: Default::default(),
    alignment: None,
    flex: 1.,
  })
  .extend(Flex::column())
  .extend(Container::size((500., 700.)))
  .lens(lens!(Todo, items))
}

#[derive(PartialEq, Clone, Default)]

pub struct Counter {
  pub count: usize,
}
