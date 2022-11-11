pub trait IncrementAble {
  type Delta;
  type Error;

  /// return reversed delta
  ///
  /// if the revered delta not actually used, I believe compiler optimization will handle this well.
  fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error>;
}

pub type DeltaOf<T> = <T as IncrementAble>::Delta;

pub enum VecDelta<T: IncrementAble> {
  Push(T),
  Remove(usize),
  Insert(usize, T),
  Pop,
}

impl<T: IncrementAble> IncrementAble for Vec<T> {
  type Delta = VecDelta<T>;
  type Error = (); // todo

  fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
    let r = match delta {
      VecDelta::Push(value) => {
        self.push(value);
        VecDelta::Pop
      }
      VecDelta::Remove(index) => {
        let item = self.remove(index);
        VecDelta::Insert(index, item)
      }
      VecDelta::Insert(index, item) => {
        self.insert(index, item);
        VecDelta::Remove(index)
      }
      VecDelta::Pop => {
        let value = self.pop().unwrap();
        VecDelta::Push(value)
      }
    };

    Ok(r)
  }
}

struct VectorMap<T, U, X> {
  mapped: X,
  mapper: Box<dyn Fn(&T) -> U>,
}

impl<T, U, X> IncrementAble for VectorMap<T, U, X>
where
  T: IncrementAble,
  X: IncrementAble<Delta = VecDelta<U>, Error = ()>,
{
  type Delta = VecDelta<T>;
  type Error = ();
  fn apply(&mut self, delta: VecDelta<T>) -> Result<Self::Delta, Self::Error> {
    match delta {
      VecDelta::Push(value) => self.mapped.apply(VecDelta::Push((self.mapper)(&value))),
      VecDelta::Remove(index) => self.mapped.apply(VecDelta::Remove(index)),
      VecDelta::Pop => self.mapped.apply(VecDelta::Pop),
      VecDelta::Insert(_, _) => todo!(),
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

impl IncrementAble for f32 {
  type Delta = Self;
  type Error = ();

  fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
    *self = delta;
    Ok(())
  }
}

impl IncrementAble for bool {
  type Delta = Self;
  type Error = ();

  fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
    *self = delta;
    Ok(())
  }
}

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

pub struct IncrementInstance<T: IncrementAble> {
  value: T,
  deltas: Vec<T::Delta>,
}

impl<T: IncrementAble> IncrementInstance<T> {
  pub fn push(&mut self, delta: T::Delta) {
    self.deltas.push(delta)
  }

  pub fn flush(&mut self) {
    self.deltas.drain(..).for_each(|d| {
      self.value.apply(d);
    })
  }
}

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
  fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
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
  fn apply(&mut self, delta: Self::Delta) -> Result<Self::Delta, Self::Error> {
    todo!()
  }
}

struct PlatformEvent;

enum ViewEvent<T: IncrementAble> {
  Platform(PlatformEvent),
  StateDelta(T::Delta),
}

/// T is state.
trait View<T>
where
  T: IncrementAble,
{
  type Event;
  fn event(&mut self, model: &mut T, event: &ViewEvent<T>) -> Option<Self::Event>;
  // fn update(&mut self, model: &T);
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

  fn event(&mut self, model: &mut T, event: &ViewEvent<T>) -> Option<Self::Event> {
    let react = false;
    // todo
    // processing platform events
    // modify self editing text, and dispatch events

    if react {
      Some(TextBoxEvent::Submit(self.texting))
    } else {
      None
    }
  }
}

struct Title<T: IncrementAble> {
  title: Box<dyn Fn(DeltaOf<T>) -> Option<String>>,
  title_current: String,
}

impl<T: IncrementAble> View<T> for Title<T> {
  type Event = ();

  fn event(&mut self, model: &mut T, event: &ViewEvent<T>) -> Option<Self::Event> {
    match event {
      ViewEvent::Platform(_) => todo!(),
      ViewEvent::StateDelta(d) => {
        if let Some(new_title) = (self.title)(d) {
          self.title_current = new_title;
        }
      }
    }
    None
  }
}

struct List<V, T> {
  views: Vec<V>,
  build_item_view: Box<dyn Fn(&T) -> V>,
}

impl<T: IncrementAble, V: View<T>> View<Vec<T>> for List<V, T> {
  type Event = V::Event;

  fn event(&mut self, model: &mut Vec<T>, event: &ViewEvent<Vec<T>>) -> Option<Self::Event> {
    let mapped_e = match event {
      ViewEvent::Platform(e) => {
        for (i, view) in self.views.iter().enumerate() {
          return view.event(model.get_mut(i).unwrap(), &ViewEvent::Platform(*e));
        }
      }
      ViewEvent::StateDelta(d) => {
        model.apply(d);
        // map d to DeltaOf<Vec<V>>, and apply!
        // use create or direct map sub delta!
      }
    };
    None
  }
}

fn todo_list_view() -> impl View<TodoList, Event = ()> {
  Container::wrap(
    TextBox::placeholder("what needs to be done?") //
      .on(submit(|value| {
        TodoListChange::List(VecDelta::Push(TodoItem {
          name: value,
          finished: false,
        }))
      })),
    List::for_by(
      |delta| matches!(delta, List),
      |event| TodoListChange::List(VecDelta::Remove(index)),
      todo_item_view,
    ),
  )
}

enum TodoItemEvent {
  DeleteSelf,
}

fn todo_item_view() -> impl View<TodoItem, Event = TodoItemEvent> {
  Container::wrap(
    Title::name(bind!(Name)),
    Toggle::status(bind!(Finished)).on(),
    Button::name("delete") //
      .on_click(|event, item| TodoItemEvent::Delete),
  )
}
