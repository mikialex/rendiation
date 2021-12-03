use std::any::Any;

pub mod counter;
pub mod todo;

use counter::*;
use todo::*;

pub struct UIExamples {
  examples: Vec<Box<dyn Any>>,
}

impl Default for UIExamples {
  fn default() -> Self {
    let mut r = Self {
      examples: Default::default(),
    };

    let todo = Todo {
      items: vec![
        TodoItem {
          name: String::from("t1中文测试"),
          id: 0,
        },
        TodoItem {
          name: String::from("test 2"),
          id: 1,
        },
        TodoItem {
          name: String::from("test gh3"),
          id: 2,
        },
      ],
    };

    r.examples.push(Box::new(todo));
    r.examples.push(Box::new(Counter::default()));
    r
  }
}

// pub fn build_ui_examples() -> impl UIComponent<UIExamples> {}
