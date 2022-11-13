use crate::*;
// todo mvc

// states
#[derive(Default, Clone)]
struct TodoItem {
  name: String,
  finished: bool,
}

/// should generate by macro
enum TodoItemChange {
  Finished(bool),
  Name(String),
}

/// should generate by macro
impl IncrementAble for TodoItem {
  type Delta = TodoItemChange;
  type Error = ();
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      TodoItemChange::Finished(v) => self.finished.apply(v)?,
      TodoItemChange::Name(v) => self.name.apply(v)?,
    }
    Ok(())
  }

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(TodoItemChange::Name(self.name.clone()));
    cb(TodoItemChange::Finished(self.finished));
  }
}

#[derive(Default, Clone)]
struct TodoList {
  list: Vec<TodoItem>,
}

/// should generate by macro
enum TodoListChange {
  List(DeltaOf<Vec<TodoItem>>),
}

/// should generate by macro
impl IncrementAble for TodoList {
  type Delta = TodoListChange;
  type Error = ();
  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    todo!()
  }
  fn expand(&self, cb: impl FnMut(Self::Delta)) {
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

struct TextBox<T: IncrementAble> {
  texting: String,
  text_binding: Box<dyn Fn(&DeltaOf<T>) -> Option<String>>,
  placeholder: Box<dyn Fn()>,
}

enum TextBoxEvent {
  Submit(String),
}

impl<T: IncrementAble> View<T> for TextBox<T> {
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
    if let Some(new) = (self.text_binding)(delta) {
      self.texting = new;
    }
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

struct List<V> {
  views: Vec<V>,
  build_item_view: Box<dyn Fn() -> V>,
}

impl<T: IncrementAble + Clone, V: View<T>> View<Vec<T>> for List<V> {
  type Event = V::Event;

  fn event(&mut self, model: &Vec<T>, event: &PlatformEvent) -> ViewDelta<Self::Event, Vec<T>> {
    for (i, view) in self.views.iter_mut().enumerate() {
      view.event(model.get(i).unwrap(), event);
    }
    ViewDelta::None
  }

  fn update(&mut self, model: &Vec<T>, delta: &DeltaOf<Vec<T>>) {
    match delta {
      VecDelta::Push(v) => {
        self.views.push((self.build_item_view)());
        let pushed = self.views.last_mut().unwrap();
        v.expand(|d| pushed.update(v, &d));
      }
      VecDelta::Remove(_) => todo!(),
      VecDelta::Insert(_, _) => todo!(),
      VecDelta::Mutate(index, d) => {
        let v = model.get(*index).unwrap();
        let view = self.views.get_mut(*index).unwrap();
        view.update(v, d)
      }
      VecDelta::Pop => {
        self.views.pop();
      }
    }
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
