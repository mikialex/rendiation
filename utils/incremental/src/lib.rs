use std::fmt::Debug;

pub mod rev;

pub trait IncrementAble {
  type Delta;
  type Error: Debug;

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error>;
}

pub type DeltaOf<T> = <T as IncrementAble>::Delta;

pub enum VecDelta<T: IncrementAble> {
  Push(T),
  Remove(usize),
  Insert(usize, T),
  Mutate(usize, DeltaOf<T>),
  Pop,
}

impl<T: IncrementAble> IncrementAble for Vec<T> {
  type Delta = VecDelta<T>;
  type Error = (); // todo

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      VecDelta::Push(value) => {
        self.push(value);
      }
      VecDelta::Remove(index) => {
        self.remove(index);
      }
      VecDelta::Insert(index, item) => {
        self.insert(index, item);
      }
      VecDelta::Pop => {
        self.pop().unwrap();
      }
      VecDelta::Mutate(index, delta) => {
        let inner = self.get_mut(index).unwrap();
        inner.apply(delta).unwrap();
      }
    };
    Ok(())
  }
}

struct VectorMap<T: IncrementAble, U: IncrementAble, X> {
  mapped: X,
  mapper: Box<dyn Fn(&T) -> U>,
  map_delta: Box<dyn Fn(&DeltaOf<T>) -> DeltaOf<U>>,
}

impl<T, U, X> IncrementAble for VectorMap<T, U, X>
where
  T: IncrementAble<Error = ()>,
  U: IncrementAble<Error = ()>,
  X: IncrementAble<Delta = VecDelta<U>, Error = ()>,
{
  type Delta = VecDelta<T>;
  type Error = ();
  fn apply(&mut self, delta: VecDelta<T>) -> Result<(), Self::Error> {
    match delta {
      VecDelta::Push(value) => self.mapped.apply(VecDelta::Push((self.mapper)(&value))),
      VecDelta::Remove(index) => self.mapped.apply(VecDelta::Remove(index)),
      VecDelta::Pop => self.mapped.apply(VecDelta::Pop),
      VecDelta::Insert(index, value) => self
        .mapped
        .apply(VecDelta::Insert(index, (self.mapper)(&value))),
      VecDelta::Mutate(index, delta) => self
        .mapped
        .apply(VecDelta::Mutate(index, (self.map_delta)(&delta))),
    }
  }
}

// struct VectorFilter<T, X> {
//   mapped: X,
//   raw_max: usize,
//   filtered_index: std::collections::HashSet<usize>,
//   filter: Box<dyn Fn(&T) -> bool>,
// }

// impl<T, X> IncrementAble for VectorFilter<T, X>
// where
//   X: IncrementAble<Delta = VecDelta<T>>,
// {
//   type Delta = VecDelta<T>;
//   fn apply(&mut self, delta: VecDelta<T>) {
//     match delta {
//       VecDelta::Push(value) => {
//         if (self.filter)(&value) {
//           self.mapped.apply(VecDelta::Push(value));
//         } else {
//           self.filtered_index.insert(self.raw_max);
//         }
//         self.raw_max += 1;
//       }
//       VecDelta::Remove(index) => {
//         if self.filtered_index.remove(&index) {
//           self.mapped.apply(VecDelta::Remove(todo!()));
//         }
//         self.raw_max -= 1
//       }
//       VecDelta::Pop => {
//         if self.filtered_index.remove(&self.raw_max) {
//           self.mapped.apply(VecDelta::Pop);
//         }
//         self.raw_max -= 1
//       }
//     }
//   }
// }

// struct Test {
//   a: f32,
//   b: bool,
// }

// enum TestIncremental {
//   A(DeltaOf<f32>),
//   B(DeltaOf<bool>),
// }

// impl IncrementAble for Test {
//   type Delta = TestIncremental;

//   fn apply(&mut self, delta: Self::Delta) {
//     match delta {
//       TestIncremental::A(v) => self.a.apply(v),
//       TestIncremental::B(v) => self.b.apply(v),
//     }
//   }
// }

// todo mvc

// states

struct TodoItem {
  name: String,
  finished: bool,
}

enum TodoItemChange {
  Finished(bool),
  Name(String),
}

impl IncrementAble for TodoItem {
  type Delta = TodoItemChange;
  type Error = ();
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    todo!()
  }
}

struct TodoList {
  list: Vec<TodoItem>,
}

enum TodoListChange {
  List(DeltaOf<Vec<TodoItem>>),
}

impl IncrementAble for TodoList {
  type Delta = TodoListChange;
  type Error = ();
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    todo!()
  }
}

struct PlatformEvent;

enum ViewDelta<V, T: IncrementAble> {
  ViewEvent(V),
  StateDelta(T::Delta),
  None,
}

/// T is state.
trait View<T>
where
  T: IncrementAble,
{
  type Event;
  fn event(&mut self, model: &T, event: &PlatformEvent) -> ViewDelta<Self::Event, T>;
  fn update(&mut self, model: &T, delta: &T::Delta);
}

struct TextBox {
  texting: String,
  placeholder: Box<dyn Fn()>,
}

enum TextBoxEvent {
  Submit(String),
}

impl<T: IncrementAble> View<T> for TextBox {
  type Event = TextBoxEvent;

  fn event(&mut self, model: &T, event: &PlatformEvent) -> ViewDelta<Self::Event, T> {
    let react = false;
    // omit
    // processing platform events
    // modify self editing text, and dispatch events

    if react {
      ViewDelta::ViewEvent(TextBoxEvent::Submit(self.texting.clone()))
    } else {
      ViewDelta::None
    }
  }
  fn update(&mut self, model: &T, delta: &T::Delta) {
    todo!()
  }
}

struct Title<T: IncrementAble> {
  title: Box<dyn Fn(&DeltaOf<T>) -> Option<String>>,
  title_current: String,
}

impl<T: IncrementAble> View<T> for Title<T> {
  type Event = ();

  fn event(&mut self, model: &T, event: &PlatformEvent) -> ViewDelta<Self::Event, T> {
    ViewDelta::None
  }
  fn update(&mut self, model: &T, delta: &T::Delta) {
    if let Some(new_title) = (self.title)(delta) {
      self.title_current = new_title;
    }
  }
}

struct List<V, T> {
  views: Vec<V>,
  build_item_view: Box<dyn Fn(&T) -> V>,
}

impl<T: IncrementAble, V: View<T>> View<Vec<T>> for List<V, T> {
  type Event = V::Event;

  fn event(&mut self, model: &Vec<T>, event: &PlatformEvent) -> ViewDelta<Self::Event, Vec<T>> {
    for (i, view) in self.views.iter_mut().enumerate() {
      view.event(model.get(i).unwrap(), event);
    }
    ViewDelta::None
  }

  fn update(&mut self, model: &Vec<T>, delta: &DeltaOf<Vec<T>>) {
    todo!()
    // map d to DeltaOf<Vec<V>>, and apply!
    // use create or direct map sub delta!
  }
}

// fn todo_list_view() -> impl View<TodoList, Event = ()> {
//   Container::wrap(
//     TextBox::placeholder("what needs to be done?") //
//       .on(submit(|value| {
//         TodoListChange::List(VecDelta::Push(TodoItem {
//           name: value,
//           finished: false,
//         }))
//       })),
//     List::for_by(
//       |delta| matches!(delta, List),
//       |event| TodoListChange::List(VecDelta::Remove(index)),
//       todo_item_view,
//     ),
//   )
// }

// enum TodoItemEvent {
//   DeleteSelf,
// }

// fn todo_item_view() -> impl View<TodoItem, Event = TodoItemEvent> {
//   Container::wrap(
//     Title::name(bind!(Name)),
//     Toggle::status(bind!(Finished)).on(),
//     Button::name("delete") //
//       .on_click(|event, item| TodoItemEvent::Delete),
//   )
// }
