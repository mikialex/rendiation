use crate::*;
// todo mvc

pub struct PlatformEvent;

pub enum ViewReaction<V, T: IncrementAble> {
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
  fn event(
    &mut self,
    model: &T,
    event: &PlatformEvent,
    cb: impl FnMut(ViewReaction<Self::Event, T>),
  );

  /// update is responsible for map the state delta to to view property change
  /// the model here is the unmodified.
  fn update(&mut self, model: &T, delta: &T::Delta);
}

// trait ViewDyn<T>
// where
//   T: IncrementAble,
// {
//   type Event;

//   fn event(
//     &mut self,
//     model: &T,
//     event: &PlatformEvent,
//     cb: &dyn FnMut(ViewReaction<Self::Event, T>),
//   );

//   /// update is responsible for map the state delta to to view property change
//   /// the model here is the unmodified.
//   fn update(&mut self, model: &T, delta: &T::Delta);
// }

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
  placeholder: String,
}

impl<T: IncrementAble> TextBox<T> {
  pub fn placeholder(placeholder: impl Into<String>) -> Self {
    Self {
      texting: Default::default(),
      text_binding: Box::new(|d| None),
      placeholder: placeholder.into(),
    }
  }
  pub fn with_text(mut self, binder: impl Fn(&DeltaOf<T>) -> Option<&String> + 'static) -> Self {
    self.text_binding = Box::new(binder);
    self
  }
}

fn _test(text: TextBox<TodoItem>) {
  text.with_text(bind!(DeltaOf::<TodoItem>::Name));
}

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
    mut cb: impl FnMut(ViewReaction<Self::Event, T>),
  ) {
    let react = false;
    // omit
    // processing platform events
    // modify self editing text, and dispatch events

    if react {
      cb(ViewReaction::ViewEvent(TextBoxEvent::Submit(
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

  fn event(
    &mut self,
    model: &T,
    event: &PlatformEvent,
    cb: impl FnMut(ViewReaction<Self::Event, T>),
  ) {
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
    mut cb: impl FnMut(ViewReaction<Self::Event, Vec<T>>),
  ) {
    for (i, view) in self.views.iter_mut().enumerate() {
      view.event(model.get(i).unwrap(), event, |e| {
        cb(match e {
          ViewReaction::ViewEvent(e) => {
            ViewReaction::ViewEvent(EventWithIndex { index: i, event: e })
          }
          ViewReaction::StateDelta(delta) => ViewReaction::StateDelta(VecDelta::Mutate(i, delta)),
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

struct EventHandler<T: IncrementAble, V: View<T>> {
  inner: V,
  handler: Box<dyn Fn(&T, &ViewReaction<V::Event, T>) -> Option<T::Delta>>,
}
trait WrapEventHandler<T: IncrementAble>: View<T> + Sized {
  fn on(
    self,
    handler: impl Fn(&T, &ViewReaction<Self::Event, T>) -> Option<T::Delta> + 'static,
  ) -> EventHandler<T, Self>;
}
impl<T: IncrementAble, V: View<T>> WrapEventHandler<T> for V {
  fn on(
    self,
    handler: impl Fn(&T, &ViewReaction<Self::Event, T>) -> Option<T::Delta> + 'static,
  ) -> EventHandler<T, Self> {
    EventHandler {
      inner: self,
      handler: Box::new(handler),
    }
  }
}

impl<T, V> View<T> for EventHandler<T, V>
where
  T: IncrementAble,
  V: View<T>,
{
  type Event = V::Event;

  fn event(
    &mut self,
    model: &T,
    event: &PlatformEvent,
    mut cb: impl FnMut(ViewReaction<Self::Event, T>),
  ) {
    self.inner.event(model, event, |react| {
      if let Some(new_delta) = (self.handler)(model, &react) {
        cb(ViewReaction::StateDelta(new_delta));
      }
      cb(react)
    })
  }

  fn update(&mut self, model: &T, delta: &T::Delta) {
    self.inner.update(model, delta)
  }
}

/// The actual state holder
struct ViewRoot<T: IncrementAble, V> {
  state: T,
  state_mutations: Vec<T::Delta>,
  view: V,
}

impl<T, V> View<()> for ViewRoot<T, V>
where
  T: IncrementAble,
  V: View<T>,
{
  type Event = V::Event;

  fn event(
    &mut self,
    _: &(),
    event: &PlatformEvent,
    mut cb: impl FnMut(ViewReaction<Self::Event, ()>),
  ) {
    self.view.event(&self.state, event, |e| match e {
      ViewReaction::StateDelta(delta) => self.state_mutations.push(delta),
      ViewReaction::ViewEvent(e) => cb(ViewReaction::ViewEvent(e)),
    });
  }

  fn update(&mut self, _: &(), _: &()) {
    for delta in self.state_mutations.drain(..) {
      self.view.update(&self.state, &delta);
      self.state.apply(delta).unwrap()
    }
  }
}

// struct Container<T> {
//   dyn_views: Vec<Box<dyn View<T, Event = ()>>>,
// }

/// library util
fn submit<T: IncrementAble>(
  on_submit: impl Fn(String) -> Option<T::Delta>,
) -> impl Fn(&T, &ViewReaction<TextBoxEvent, T>) -> Option<T::Delta> {
  move |_, e| match e {
    ViewReaction::ViewEvent(e) => match e {
      TextBoxEvent::Submit(content) => on_submit(content.clone()),
    },
    ViewReaction::StateDelta(_) => None,
  }
}

fn todo_list_view() -> impl View<TodoList> {
  TextBox::placeholder("what needs to be done?") //
    .on(submit(|text| {
      TodoListChange::List(VecDelta::Push(TodoItem {
        name: text,
        finished: false,
      }))
      .into()
    }))
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
