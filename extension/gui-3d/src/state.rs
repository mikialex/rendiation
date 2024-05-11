#![allow(clippy::missing_safety_doc)]

use std::ops::{Deref, DerefMut};

use crate::*;

pub struct MessageStore {
  messages: FastHashMap<TypeId, Box<dyn Any>>,
}

impl MessageStore {
  pub fn put(&mut self, msg: impl Any) {
    //
  }
  pub fn take<T>(&mut self) -> Option<T> {
    todo!()
  }
}

pub struct StateStore {
  states: FastHashMap<TypeId, Vec<*mut ()>>,
}

pub struct StateGuard<'a, T> {
  pub _unique_life: &'a (),
  pub ptr: *mut T,
}

impl<'a, T> Deref for StateGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { self.ptr.as_ref().unwrap() }
  }
}

#[macro_export]
macro_rules! state_access {
  ($store: expr, $name: tt, $type: ty) => {
    let $name = unsafe { $store.get_state_ptr::<$type>() };
    #[allow(unused_variables)]
    let $name = StateGuard {
      _unique_life: &(),
      ptr: $name,
    };
    let $name: &$type = &$name;
  };
}

pub struct StateMutGuard<'a, T> {
  pub _unique_life: &'a mut (),
  pub ptr: *mut T,
}

impl<'a, T> Deref for StateMutGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { self.ptr.as_ref().unwrap() }
  }
}
impl<'a, T> DerefMut for StateMutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { self.ptr.as_mut().unwrap() }
  }
}

#[macro_export]
macro_rules! state_mut_access {
  ($store: expr, $name: tt, $type: ty) => {
    let $name = unsafe { $store.get_state_ptr::<$type>() };
    #[allow(unused_variables)]
    let mut $name = StateMutGuard {
      _unique_life: &mut (),
      ptr: $name,
    };
    let $name: &mut $type = &mut $name;
  };
}

impl StateStore {
  pub unsafe fn get_state_raw<T: 'static>(&mut self) -> &mut T {
    self.get_state_ptr::<T>().as_mut().unwrap()
  }

  pub unsafe fn get_state_ptr<T: 'static>(&mut self) -> *mut T {
    let last_ptr = self
      .states
      .entry(TypeId::of::<T>())
      .or_default()
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

impl<T: 'static, V: View> View for StateCtxInject<T, V> {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    unsafe {
      cx.state.register_state(&mut self.state);
      self.view.update_view(cx);
      cx.state.unregister_state::<T>()
    }
  }

  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    unsafe {
      cx.state.register_state(&mut self.state);
      self.view.update_state(cx);
      cx.state.unregister_state::<T>()
    }
  }
}

pub struct StateCtxPick<V, F, T1, T2> {
  pub view: V,
  pub pick: F,
  pub phantom: PhantomData<(T1, T2)>,
}

impl<T1: 'static, T2: 'static, F: Fn(&mut T1) -> &mut T2, V: View> View
  for StateCtxPick<V, F, T1, T2>
{
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    unsafe {
      let s = cx.state.get_state_ptr::<T1>();
      let picked = (self.pick)(s.as_mut().unwrap());

      cx.state.register_state(picked);
      self.view.update_view(cx);
      cx.state.unregister_state::<T2>()
    }
  }

  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    unsafe {
      let s = cx.state.get_state_ptr::<T1>();
      let picked = (self.pick)(s.as_mut().unwrap());

      cx.state.register_state(picked);
      self.view.update_state(cx);
      cx.state.unregister_state::<T2>()
    }
  }
}
