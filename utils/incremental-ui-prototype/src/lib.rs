#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use std::{
  any::Any,
  cell::RefCell,
  marker::PhantomData,
  rc::{Rc, Weak},
  sync::{Arc, RwLock},
};

use incremental::*;
// todo mvc

pub struct PlatformEvent;

pub enum ViewReaction<V, T: Incremental> {
  /// emit self special event
  ViewEvent(V),
  /// do state mutation
  StateDelta(T::Delta),
}

enum PlatformRequest<'a, T: Incremental, V: View<T>> {
  Event {
    event: &'a PlatformInput,
    cb: &'a mut dyn FnMut(ViewReaction<V::Event, T>),
  },
  Layout {
    parent_constraint: usize,
    cb: &'a dyn FnOnce(usize),
  },
  Rendering {
    ctx: &'a usize,
  },
}

pub enum PlatformInput {}

trait ViewBase {
  fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn ViewBase));

  fn process(&mut self, name: usize) {
    self.visit_children(&mut |child| child.process(name));
  }
}

/// View type could generic over any state T, as long as the T could provide
/// given logic for view type
trait View<T>
where
  T: Incremental,
{
  /// View type's own event type
  type Event;

  /// In event loop handling, the view type received platform event such as mouse move keyboard events,
  /// and decide should reactive to it or not, if so, mutate the model or emit
  /// the self::Event for further outer side handling. see ViewDelta.
  ///
  /// all mutation to the model should record delta by call cb passed from caller.
  ///
  /// In View hierarchy, event's mutation to state will pop up to the root, wrap the mutation to
  /// parent state's delta type. and in update logic, consumed from the root
  fn event(
    &mut self,
    model: &mut T,
    event: &PlatformEvent,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, T>),
  );

  /// update is responsible for map the state delta to to view property change
  /// the model here is the unmodified.
  fn update(&mut self, model: &T, delta: &T::Delta);
}

// states
#[derive(Default, Clone, Incremental)]
pub struct TodoItem {
  pub name: String,
  pub finished: bool,
  test: Arc<RwLock<usize>>,
}

#[derive(Default, Clone, Incremental)]
pub struct TodoList {
  pub list: Vec<TodoItem>,
}

struct TextBox<T: Incremental> {
  texting: String,
  text_binding: Box<dyn Fn(&DeltaOf<T>) -> Option<&String>>,
  placeholder: String,
}

impl<T: Incremental> TextBox<T> {
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
  text.with_text(bind!(DeltaOf::<TodoItem>::name));
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

impl<T: Incremental> View<T> for TextBox<T> {
  type Event = TextBoxEvent;

  fn event(
    &mut self,
    model: &mut T,
    event: &PlatformEvent,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, T>),
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
    if let Some(new) = (self.text_binding)(delta) {
      self.texting = new.clone();
    }
  }
}

struct Title<T: Incremental> {
  title: Box<dyn Fn(&DeltaOf<T>) -> Option<&String>>,
  title_current: String,
}

impl<T: Incremental> Title<T> {
  pub fn name(binder: impl Fn(&DeltaOf<T>) -> Option<&String> + 'static) -> Self {
    Self {
      title: Box::new(binder),
      title_current: Default::default(),
    }
  }
}

impl<T: Incremental> View<T> for Title<T> {
  type Event = ();

  fn event(
    &mut self,
    model: &mut T,
    event: &PlatformEvent,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, T>),
  ) {
  }
  fn update(&mut self, model: &T, delta: &T::Delta) {
    if let Some(new_title) = (self.title)(delta) {
      self.title_current = new_title.clone();
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

impl<T, V> View<Vec<T>> for List<V>
where
  T: Incremental + Default + Send + Sync + Clone + 'static,
  V: View<T>,
{
  type Event = EventWithIndex<V::Event>;

  fn event(
    &mut self,
    model: &mut Vec<T>,
    event: &PlatformEvent,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, Vec<T>>),
  ) {
    for (i, view) in self.views.iter_mut().enumerate() {
      view.event(model.get_mut(i).unwrap(), event, &mut |e| {
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

struct EventHandler<T: IncrementalMutatorHelper + 'static, V: View<T>> {
  inner: V,
  handler: Box<dyn for<'a> Fn(T::Mutator<'a>, &ViewReaction<V::Event, T>)>,
}
trait WrapEventHandler<T: IncrementalMutatorHelper>: View<T> + Sized {
  fn on(
    self,
    handler: impl for<'a> Fn(T::Mutator<'a>, &ViewReaction<Self::Event, T>) + 'static,
  ) -> EventHandler<T, Self>;
}
impl<T: IncrementalMutatorHelper, V: View<T>> WrapEventHandler<T> for V {
  fn on(
    self,
    handler: impl for<'a> Fn(T::Mutator<'a>, &ViewReaction<Self::Event, T>) + 'static,
  ) -> EventHandler<T, Self> {
    EventHandler {
      inner: self,
      handler: Box::new(handler),
    }
  }
}

impl<T, V> View<T> for EventHandler<T, V>
where
  T: IncrementalMutatorHelper,
  V: View<T>,
{
  type Event = V::Event;

  fn event(
    &mut self,
    model: &mut T,
    event: &PlatformEvent,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, T>),
  ) {
    let mut reaction = Vec::new(); // todo optimize use small vec
    self.inner.event(model, event, &mut |react| {
      reaction.push(react);
    });

    reaction.drain(..).for_each(|react| {
      (self.handler)(
        model.create_mutator(&mut |d| cb(ViewReaction::StateDelta(d))),
        &react,
      )
    });
  }

  fn update(&mut self, model: &T, delta: &T::Delta) {
    self.inner.update(model, delta)
  }
}

/// The actual state holder
struct ViewRoot<T: Incremental, V> {
  state: T,
  state_mutations: Vec<T::Delta>,
  view: V,
}

impl<T, V> View<()> for ViewRoot<T, V>
where
  T: Incremental,
  V: View<T>,
{
  type Event = V::Event;

  fn event(
    &mut self,
    _: &mut (),
    event: &PlatformEvent,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, ()>),
  ) {
    self.view.event(&mut self.state, event, &mut |e| match e {
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

struct Container<T, E> {
  dyn_views: Vec<Box<dyn View<T, Event = E>>>,
}

impl<T, E> Default for Container<T, E> {
  fn default() -> Self {
    Self {
      dyn_views: Default::default(),
    }
  }
}

impl<T: Incremental, E> Container<T, E> {
  pub fn with_child(mut self, view: impl View<T, Event = E> + 'static) -> Self {
    self.dyn_views.push(Box::new(view));
    self
  }
}

impl<T: Incremental, E> View<T> for Container<T, E> {
  type Event = E;

  fn event(
    &mut self,
    model: &mut T,
    event: &PlatformEvent,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, T>),
  ) {
    for view in &mut self.dyn_views {
      view.event(model, event, cb)
    }
  }

  fn update(&mut self, model: &T, delta: &T::Delta) {
    for view in &mut self.dyn_views {
      view.update(model, delta)
    }
  }
}

/// library util
fn submit<T: IncrementalMutatorHelper>(
  on_submit: impl Fn(String) -> Option<T::Delta>,
) -> impl for<'a> Fn(T::Mutator<'a>, &ViewReaction<TextBoxEvent, T>) {
  move |mut mutator, e| match e {
    ViewReaction::ViewEvent(e) => match e {
      TextBoxEvent::Submit(content) => {
        if let Some(delta) = on_submit(content.clone()) {
          // mutator.apply(delta);
        }
      }
    },
    ViewReaction::StateDelta(_) => {}
  }
}

fn todo_list_view() -> impl View<TodoList> {
  Container::default() //
    .with_child(
      TextBox::placeholder("what needs to be done?") //
        .on(submit(|text| {
          TodoListDelta::list(VecDelta::Push(TodoItem {
            name: text,
            finished: false,
            test: Arc::new(RwLock::new(1)),
          }))
          .into()
        })),
    )
  // .with_child(
  //   List::for_by(todo_item_view), //
  // )
}

impl IncrementalMutatorHelper for TodoList {
  type Mutator<'a> = ()
  where
    Self: 'a;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    todo!()
  }
}

// fn todo_list_view() -> impl View<TodoList, Event = ()> {
//   Container::wrap(
//     TextBox::placeholder("what needs to be done?") //
//       .on(submit(|value| {
//         TodoListDelta::List(VecDelta::Push(TodoItem {
//           name: value,
//           finished: false,
//         }))
//       })),
//     List::for_by(todo_item_view)
//       .lens(lens!(TodoList::list))
//       .on(inner(|event| TodoListDelta::List(VecDelta::Remove(event.index)))),
//   )
// }

enum TodoItemEvent {
  DeleteSelf,
}

fn todo_item_view() -> impl View<TodoItem, Event = ()> {
  Container::default().with_child(
    Title::name(bind!(DeltaOf::<TodoItem>::name)),
    // Toggle::status(bind!(Finished)).on(),
    // Button::name("delete") //
    //   .on_click(|event, item| TodoItemEvent::Delete),
  )
}

struct UISystem {
  view_roots: Vec<Weak<RefCell<dyn View<(), Event = ()>>>>,
}

impl UISystem {
  pub fn event(&mut self, event: &PlatformEvent) {
    for view in &self.view_roots {
      // view.borrow_mut().event(&mut (), event, &mut |_| {})
    }
  }

  pub fn add_view_root<T: Incremental, V: View<T>>(&mut self, root: ViewRoot<T, V>) {
    //
  }
}
