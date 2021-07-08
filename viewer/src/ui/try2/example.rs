use super::*;

struct Todo {
  items: TodoItems,
}

struct TodoItems {
  name: String,
}

// fn build_todo() -> impl Component<Todo> {
//   Flex::<Todo> {
//     children: Vec::new(),
//   }
// }
