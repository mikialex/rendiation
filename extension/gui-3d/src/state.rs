#![allow(clippy::missing_safety_doc)]

use std::ops::{Deref, DerefMut};

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
pub struct StateCx {
  pub message: MessageStore,
  states: FastHashMap<TypeId, Vec<*mut ()>>,
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

impl StateCx {
  pub fn split_state<T>(&mut self, f: impl FnOnce(&mut T, &mut Self)) {
    todo!();
  }

  pub unsafe fn get_state_ref<T: 'static>(&self) -> &T {
    self.get_state_ptr::<T>().as_ref().unwrap()
  }
  pub unsafe fn get_state_mut<T: 'static>(&mut self) -> &mut T {
    self.get_state_ptr::<T>().as_mut().unwrap()
  }

  pub unsafe fn get_state_ptr<T: 'static>(&self) -> *mut T {
    let last_ptr = self
      .states
      .get(&TypeId::of::<T>())
      .unwrap()
      .last()
      .cloned()
      .unwrap();

    last_ptr as *mut T
  }

  pub unsafe fn register_state<T: 'static>(&mut self, v: &mut T) {
    self
      .states
      .entry(TypeId::of::<T>())
      .or_default()
      .push(v as *mut T as *mut ())
  }

  pub unsafe fn unregister_state<T: 'static>(&mut self) {
    self
      .states
      .entry(TypeId::of::<T>())
      .or_default()
      .pop()
      .unwrap();
  }
}

pub struct StateCtxInject<T, V> {
  pub view: V,
  pub state: T,
}

impl<T: 'static, V: StatefulView> StatefulView for StateCtxInject<T, V> {
  fn update_view(&mut self, cx: &mut StateCx) {
    unsafe {
      cx.register_state(&mut self.state);
      self.view.update_view(cx);
      cx.unregister_state::<T>()
    }
  }

  fn update_state(&mut self, cx: &mut StateCx) {
    unsafe {
      cx.register_state(&mut self.state);
      self.view.update_state(cx);
      cx.unregister_state::<T>()
    }
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

impl<T1: 'static, T2: 'static, F: Fn(&mut T1) -> &mut T2, V: StatefulView> StatefulView
  for StateCtxPick<V, F, T1, T2>
{
  fn update_view(&mut self, cx: &mut StateCx) {
    unsafe {
      let s = cx.get_state_ptr::<T1>();
      let picked = (self.pick)(s.as_mut().unwrap());

      cx.register_state(picked);
      self.view.update_view(cx);
      cx.unregister_state::<T2>()
    }
  }

  fn update_state(&mut self, cx: &mut StateCx) {
    unsafe {
      let s = cx.get_state_ptr::<T1>();
      let picked = (self.pick)(s.as_mut().unwrap());

      cx.register_state(picked);
      self.view.update_state(cx);
      cx.unregister_state::<T2>()
    }
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    self.view.clean_up(cx)
  }
}
