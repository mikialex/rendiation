use interphaser::*;

use crate::button;

pub struct Todo {
  pub items: Vec<TodoItem>,
}

#[derive(Clone, PartialEq)]
pub struct TodoItem {
  pub name: String,
}

// todo change to id
impl IdentityKeyed for TodoItem {
  type Key = String;

  fn key(&self) -> Self::Key {
    self.name.clone()
  }
}

pub fn build_todo() -> impl UIComponent<Todo> {
  For::by(|item: &TodoItem, i| Child::flex(build_todo_item(), 1.))
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
pub type TodoItemDeleteHandler<T> = EventHandler<T, TodoItemDelete>;
impl EventHandlerType for TodoItemDelete {
  type Event = TodoItemDeleteEvent;
}
impl<C> EventHandlerImpl<C> for TodoItemDelete {
  fn downcast_event<'a>(&mut self, event: &'a mut EventCtx, inner: &C) -> Option<&'a Self::Event> {
    event
      .custom_event
      .consume_if_type_is::<TodoItemDeleteEvent>()
  }
  fn should_handle_in_bubble(&self) -> bool {
    true
  }
}

pub fn build_todo_item() -> impl UIComponent<TodoItem> {
  let label = Text::default()
    .editable()
    .bind(move |s, t: &TodoItem| s.content.set(t.name.clone()))
    .extend(Container::size((200., 100.)));

  let button = button("delete", |s: &mut TodoItem, c, _| {
    println!("delete {}", s.name);
    c.emit(TodoItemDeleteEvent {
      name: s.name.clone(),
    })
  });

  flex_group()
    .child(Child::flex(label, 1.))
    .child(Child::flex(button, 1.))
    .extend(Flex::row())
    .extend(Container::size((500., 120.)))
}

#[derive(PartialEq, Clone, Default)]

pub struct Counter {
  pub count: usize,
}
