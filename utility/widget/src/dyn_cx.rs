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
  pub fn get<T: Any>(&self) -> Option<&T> {
    self
      .messages
      .get(&TypeId::of::<T>())
      .as_ref()
      .map(|v| v.downcast_ref::<T>().unwrap())
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

pub struct CxGuard<'a, T> {
  pub ptr: &'a T,
}

impl<T> Deref for CxGuard<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.ptr
  }
}

#[macro_export]
macro_rules! access_cx {
  ($store: expr, $name: tt, $type: ty) => {
    let $name = unsafe { $store.get_cx_ref::<$type>() };
    #[allow(unused_variables)]
    let $name = CxGuard { ptr: $name };
    let $name: &$type = &$name;
  };
}

pub struct CxMutGuard<'a, T> {
  pub ptr: &'a mut T,
}

impl<T> Deref for CxMutGuard<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.ptr
  }
}
impl<T> DerefMut for CxMutGuard<'_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.ptr
  }
}

#[macro_export]
macro_rules! access_cx_mut {
  ($store: expr, $name: tt, $type: ty) => {
    let $name = unsafe { $store.get_cx_mut::<$type>() };
    #[allow(unused_variables)]
    let mut $name = CxMutGuard { ptr: $name };
    let $name: &mut $type = &mut $name;
  };
}

#[derive(Default)]
pub struct DynCx {
  pub message: MessageStore,
  cx_stack: smallvec::SmallVec<[Option<StatePtrStack>; 8]>,
  type_idx: TypeIndexRegistry,
}

type StatePtrStack = smallvec::SmallVec<[*mut (); 2]>;

impl DynCx {
  pub fn split_cx<T: 'static>(&mut self, f: impl FnOnce(&mut T, &mut Self)) {
    let ptr = self.try_pop_cx::<T>().unwrap();
    unsafe {
      f(&mut *ptr, self);
      self.register_cx(ptr);
    }
  }

  pub unsafe fn get_cx_ref<T: 'static>(&self) -> &T {
    if let Some(ptr) = self.get_cx_ptr::<T>() {
      &*ptr
    } else {
      panic!(
        "dyn cx access failed, {} typed cx not exist",
        std::any::type_name::<T>()
      )
    }
  }
  pub unsafe fn get_cx_mut<T: 'static>(&mut self) -> &mut T {
    if let Some(ptr) = self.get_cx_ptr::<T>() {
      &mut *ptr
    } else {
      panic!(
        "dyn cx access failed, {} typed cx not exist",
        std::any::type_name::<T>()
      )
    }
  }

  pub fn get_cx_ptr<T: 'static>(&self) -> Option<*mut T> {
    let idx = self.type_idx.get_ty::<T>()?;
    let ptr_stack = self.cx_stack.get(idx)?.as_ref()?;
    let last_ptr = ptr_stack.last().cloned()?;

    Some(last_ptr as *mut T)
  }

  fn get_ptr_stack<T: 'static>(&mut self) -> Option<&mut StatePtrStack> {
    let idx = self.type_idx.get_or_register_ty::<T>();

    while self.cx_stack.len() <= idx {
      self.cx_stack.push(None)
    }

    let ptr_stack = self
      .cx_stack
      .get_mut(idx)?
      .get_or_insert_with(smallvec::SmallVec::new);

    Some(ptr_stack)
  }

  pub unsafe fn register_cx<T: 'static>(&mut self, v: *mut T) {
    self.get_ptr_stack::<T>().unwrap().push(v as *mut ())
  }

  pub unsafe fn unregister_cx<T: 'static>(&mut self) -> *mut T {
    self.get_ptr_stack::<T>().unwrap().pop().unwrap_unchecked() as *mut T
  }

  pub fn try_pop_cx<T: 'static>(&mut self) -> Option<*mut T> {
    Some(self.get_ptr_stack::<T>()?.pop()? as *mut T)
  }

  pub fn scoped_cx<T: 'static>(&mut self, state: &mut T, f: impl FnOnce(&mut DynCx)) {
    unsafe {
      self.register_cx(state);
      f(self);
      self.unregister_cx::<T>();
    }
  }
}

#[test]
fn test_state_cx() {
  let mut cx = DynCx::default();

  let mut a: usize = 1;
  let mut b: usize = 2;

  unsafe {
    cx.register_cx(&mut a);
    assert_eq!(*cx.get_cx_ref::<usize>(), 1);

    cx.register_cx(&mut b);
    assert_eq!(*cx.get_cx_ref::<usize>(), 2);

    *cx.get_cx_mut::<usize>() = 3;
    assert_eq!(*cx.get_cx_ref::<usize>(), 3);

    cx.unregister_cx::<usize>();
    assert_eq!(*cx.get_cx_ref::<usize>(), 1);

    cx.unregister_cx::<usize>();
    assert!(cx.get_cx_ptr::<usize>().is_none());

    cx.message.put(a);
    assert_eq!(cx.message.take::<usize>(), Some(1));
    assert!(cx.message.take::<usize>().is_none());
  }
}
