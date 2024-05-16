#![allow(clippy::missing_safety_doc)]

use std::{
  any::{Any, TypeId},
  ops::{Deref, DerefMut},
};

use crate::*;

#[derive(Default)]
pub struct MessageStore {
  messages: FastHashMap<TypeId, Box<dyn Any>>,
}

impl MessageStore {
  pub fn put(&mut self, msg: impl Any) {
    self.messages.insert(msg.type_id(), Box::new(msg));
  }
  pub fn take<T: Any>(&mut self) -> Option<T> {
    self
      .messages
      .remove(&TypeId::of::<T>())
      .map(|v| *v.downcast::<T>().unwrap())
  }
}

#[derive(Default)]
pub struct TypeIndexRegistry {
  type_idx: FastHashMap<TypeId, usize>,
  next: usize,
}

impl TypeIndexRegistry {
  pub fn get_ty<T: Any>(&self) -> Option<usize> {
    self.type_idx.get(&TypeId::of::<T>()).cloned()
  }
  pub fn get_or_register_ty<T: Any>(&mut self) -> usize {
    let index = *self.type_idx.entry(TypeId::of::<T>()).or_insert_with(|| {
      let r = self.next;
      self.next += 1;
      r
    });
    index
  }
}

pub struct StateGuard<'a, T> {
  pub ptr: &'a T,
}

impl<'a, T> Deref for StateGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.ptr
  }
}

#[macro_export]
macro_rules! state_access {
  ($store: expr, $name: tt, $type: ty) => {
    let $name = unsafe { $store.get_state_ref::<$type>() };
    #[allow(unused_variables)]
    let $name = StateGuard { ptr: $name };
    let $name: &$type = &$name;
  };
}

pub struct StateMutGuard<'a, T> {
  pub ptr: &'a mut T,
}

impl<'a, T> Deref for StateMutGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.ptr
  }
}
impl<'a, T> DerefMut for StateMutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.ptr
  }
}

#[macro_export]
macro_rules! state_mut_access {
  ($store: expr, $name: tt, $type: ty) => {
    let $name = unsafe { $store.get_state_mut::<$type>() };
    #[allow(unused_variables)]
    let mut $name = StateMutGuard { ptr: $name };
    let $name: &mut $type = &mut $name;
  };
}

#[derive(Default)]
pub struct StateCx {
  pub message: MessageStore,
  states: smallvec::SmallVec<[Option<StatePtrStack>; 8]>,
  type_idx: TypeIndexRegistry,
}

type StatePtrStack = smallvec::SmallVec<[*mut (); 2]>;

impl StateCx {
  pub fn split_state<T: 'static>(&mut self, f: impl FnOnce(&mut T, &mut Self)) {
    unsafe {
      let ptr = self.unregister_state::<T>();
      f(&mut *ptr, self);
      self.register_state(ptr);
    }
  }

  pub unsafe fn get_state_ref<T: 'static>(&self) -> &T {
    &*self.get_state_ptr::<T>().unwrap()
  }
  pub unsafe fn get_state_mut<T: 'static>(&mut self) -> &mut T {
    &mut *self.get_state_ptr::<T>().unwrap()
  }

  pub fn get_state_ptr<T: 'static>(&self) -> Option<*mut T> {
    let idx = self.type_idx.get_ty::<T>()?;
    let ptr_stack = self.states.get(idx)?.as_ref()?;
    let last_ptr = ptr_stack.last().cloned()?;

    Some(last_ptr as *mut T)
  }

  unsafe fn get_ptr_stack<T: 'static>(&mut self) -> &mut StatePtrStack {
    let idx = self.type_idx.get_or_register_ty::<T>();

    while self.states.len() <= idx {
      self.states.push(None)
    }

    let ptr_stack = self
      .states
      .get_mut(idx)
      .unwrap_unchecked()
      .get_or_insert_with(smallvec::SmallVec::new);

    ptr_stack
  }

  pub unsafe fn register_state<T: 'static>(&mut self, v: *mut T) {
    self.get_ptr_stack::<T>().push(v as *mut ())
  }

  pub unsafe fn unregister_state<T: 'static>(&mut self) -> *mut T {
    self.get_ptr_stack::<T>().pop().unwrap_unchecked() as *mut T
  }

  pub fn state_scope<T: 'static>(&mut self, state: &mut T, f: impl FnOnce(&mut StateCx)) {
    unsafe {
      self.register_state(state);
      f(self);
      self.unregister_state::<T>();
    }
  }
}

pub struct StateCtxInject<T, V> {
  pub view: V,
  pub state: T,
}

impl<T: 'static, V: Widget> Widget for StateCtxInject<T, V> {
  fn update_view(&mut self, cx: &mut StateCx) {
    cx.state_scope(&mut self.state, |cx| {
      self.view.update_view(cx);
    })
  }

  fn update_state(&mut self, cx: &mut StateCx) {
    cx.state_scope(&mut self.state, |cx| {
      self.view.update_state(cx);
    })
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    self.view.clean_up(cx)
  }
}

pub struct StateCtxPick<V, F, T1, T2> {
  pub view: V,
  pub pick: F,
  pub phantom: PhantomData<(T1, T2)>,
}

impl<T1: 'static, T2: 'static, F: Fn(&mut T1) -> &mut T2, V: Widget> Widget
  for StateCtxPick<V, F, T1, T2>
{
  fn update_view(&mut self, cx: &mut StateCx) {
    unsafe {
      let s = cx.get_state_ptr::<T1>().unwrap();
      let picked = (self.pick)(&mut *s);

      cx.state_scope(picked, |cx| {
        self.view.update_view(cx);
      });
    }
  }

  fn update_state(&mut self, cx: &mut StateCx) {
    unsafe {
      let s = cx.get_state_ptr::<T1>().unwrap();
      let picked = (self.pick)(&mut *s);

      cx.state_scope(picked, |cx| {
        self.view.update_state(cx);
      });
    }
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    self.view.clean_up(cx)
  }
}

#[test]
fn test_state_cx() {
  let mut cx = StateCx::default();

  let mut a: usize = 1;
  let mut b: usize = 2;

  unsafe {
    cx.register_state(&mut a);
    assert_eq!(*cx.get_state_ref::<usize>(), 1);

    cx.register_state(&mut b);
    assert_eq!(*cx.get_state_ref::<usize>(), 2);

    *cx.get_state_mut::<usize>() = 3;
    assert_eq!(*cx.get_state_ref::<usize>(), 3);

    cx.unregister_state::<usize>();
    assert_eq!(*cx.get_state_ref::<usize>(), 1);

    cx.unregister_state::<usize>();
    assert!(cx.get_state_ptr::<usize>().is_none());

    cx.message.put(a);
    assert_eq!(cx.message.take::<usize>(), Some(1));
    assert!(cx.message.take::<usize>().is_none());
  }
}
