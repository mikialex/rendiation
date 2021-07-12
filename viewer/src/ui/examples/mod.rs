use super::*;

struct Todo {
  items: Vec<TodoItems>,
}

struct TodoItems {
  name: String,
}

fn build_todo() -> impl Component<Todo> {
  let r = For::by(|item, _| Text::new("test"))
    .lens(crate::lens!(Todo, items))
    .extend(Flex { direction: false });
  r
}
