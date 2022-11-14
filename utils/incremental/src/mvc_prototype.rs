use crate::*;
// todo mvc

pub struct PlatformEvent;

pub enum ViewDelta<V, T: IncrementAble> {
  /// emit self special event
  ViewEvent(V),
  /// do state mutation
  StateDelta(T::Delta),
}

/// View type could generics over any state T, as long as the T could provide
/// given logic for view type
trait View<T>
where
  T: IncrementAble,
{
  /// View type's own event type
  type Event;

  /// In event loop handling, view type received platform event such as mouse move keyboard events,
  /// and decide should reactive to it or not, if so, convert it to the mutation for model or emit
  /// the self::Event for further outer side handling. see ViewDelta.
  ///
  /// In View hierarchy, event's mutation to state will pop up to the root, wrap the mutation to
  /// parent state's delta type. and in update logic, consumed from the root
  fn event(&mut self, model: &T, event: &PlatformEvent, cb: impl FnMut(ViewDelta<Self::Event, T>));

  /// update is responsible for map the state delta to to view property change
  /// the model here is the unmodified.
  fn update(&mut self, model: &T, delta: &T::Delta);
}

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

struct TextBox<T: IncrementAble> {
  texting: String,
  text_binding: Box<dyn Fn(&DeltaOf<T>) -> Option<&String>>,
  placeholder: Box<dyn Fn()>,
}

impl<T: IncrementAble> TextBox<T> {
  pub fn with_text(mut self, binder: impl Fn(&DeltaOf<T>) -> Option<&String> + 'static) -> Self {
    self.text_binding = Box::new(binder);
    self
  }
}

fn _test(text: TextBox<TodoItem>) {
  text.with_text(bind!(DeltaOf::<TodoItem>::Name));
}

// impl<T: IncrementAble, S> DeltaBinder<T, S> for Box<dyn Fn(&DeltaOf<T>) -> Option<String>> {}

#[macro_export]
macro_rules! bind {
  ($Variant: path) => {
    |delta| {
      if let $Variant(name) = delta {
        Some(&name)
      } else {
        None
      }
    }
  };
}

enum TextBoxEvent {
  Submit(String),
}

impl<T: IncrementAble> View<T> for TextBox<T> {
  type Event = TextBoxEvent;

  fn event(
    &mut self,
    model: &T,
    event: &PlatformEvent,
    mut cb: impl FnMut(ViewDelta<Self::Event, T>),
  ) {
    let react = false;
    // omit
    // processing platform events
    // modify self editing text, and dispatch events

    if react {
      cb(ViewDelta::ViewEvent(TextBoxEvent::Submit(
        self.texting.clone(),
      )))
    }
  }
  fn update(&mut self, model: &T, delta: &T::Delta) {
    if let Some(new) = (self.text_binding)(&delta) {
      self.texting = new.clone();
    }
  }
}

struct Title<T: IncrementAble> {
  title: Box<dyn Fn(&DeltaOf<T>) -> Option<String>>,
  title_current: String,
}

impl<T: IncrementAble> View<T> for Title<T> {
  type Event = ();

  fn event(&mut self, model: &T, event: &PlatformEvent, cb: impl FnMut(ViewDelta<Self::Event, T>)) {
  }
  fn update(&mut self, model: &T, delta: &T::Delta) {
    if let Some(new_title) = (self.title)(&delta) {
      self.title_current = new_title;
    }
  }
}

struct List<V> {
  views: Vec<V>,
  build_item_view: Box<dyn Fn() -> V>,
}

impl<V> List<V> {
  pub fn for_by(view_builder: impl Fn() -> V + 'static) -> Self {
    Self {
      views: Default::default(),
      build_item_view: Box::new(view_builder),
    }
  }
}

struct EventWithIndex<T> {
  event: T,
  index: usize,
}

impl<T: IncrementAble + Default, V: View<T>> View<Vec<T>> for List<V> {
  type Event = EventWithIndex<V::Event>;

  fn event(
    &mut self,
    model: &Vec<T>,
    event: &PlatformEvent,
    mut cb: impl FnMut(ViewDelta<Self::Event, Vec<T>>),
  ) {
    for (i, view) in self.views.iter_mut().enumerate() {
      view.event(model.get(i).unwrap(), event, |e| {
        cb(match e {
          ViewDelta::ViewEvent(e) => ViewDelta::ViewEvent(EventWithIndex { index: i, event: e }),
          ViewDelta::StateDelta(delta) => ViewDelta::StateDelta(VecDelta::Mutate(i, delta)),
        })
      });
    }
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
//     List::for_by(todo_item_view)
//       .lens(lens!(TodoList::list))
//       .on(inner(|event| TodoListChange::List(VecDelta::Remove(event.index)))),
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
