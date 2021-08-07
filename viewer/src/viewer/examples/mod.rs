use interphaser::*;

use crate::button;

pub struct Todo {
  pub items: Vec<TodoItem>,
}

#[derive(Clone, PartialEq)]
pub struct TodoItem {
  pub name: String,
}

pub fn build_todo() -> impl UIComponent<Todo> {
  For::by(|item: &TodoItem, i| Child::Flex {
    widget: Box::new(build_todo_item()),
    result: Default::default(),
    position: Default::default(),
    alignment: None,
    flex: 1.,
  })
  .extend(Flex::column())
  .extend(TodoItemDeleteHandler::by(|s: &mut Vec<TodoItem>, _, e| {
    s.remove(s.iter().position(|item| item.name == e.name).unwrap());
  }))
  .extend(Container::size((800., 1000.)))
  .lens(lens!(Todo, items))
}

pub struct TodoItemDeleteEvent {
  name: String,
}

#[derive(Default)]
pub struct TodoItemDelete;
pub type TodoItemDeleteHandler<T> = EventHandler<T, TodoItemDelete, TodoItemDeleteEvent>;
impl<C> EventHandlerImpl<C> for TodoItemDelete {
  type Event = TodoItemDeleteEvent;
  fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event> {
    event.custom_event.downcast_ref::<TodoItemDeleteEvent>()
  }
  fn should_handle_in_bubble(&self) -> bool {
    true
  }
}

pub fn build_todo_item() -> impl UIComponent<TodoItem> {
  let label = Text::default()
    .bind(move |s, t: &TodoItem| s.content.set(t.name.clone()))
    .extend(Container::size((200., 100.)));

  let button = button("delete", |s: &mut TodoItem, c, _| {
    println!("delete {}", s.name);
    c.custom_event_emitter = Box::new(TodoItemDeleteEvent {
      name: s.name.clone(),
    })
  });

  ComponentArray::flex_group()
    .add_flex_child(label, 1.0, None)
    .add_flex_child(button, 1.0, None)
    .extend(Flex::row())
    .extend(Container::size((500., 120.)))
}

#[derive(PartialEq, Clone, Default)]

pub struct Counter {
  pub count: usize,
}
