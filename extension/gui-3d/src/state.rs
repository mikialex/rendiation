#![allow(clippy::missing_safety_doc)]

use crate::*;

pub struct StateStore {
  states: FastHashMap<TypeId, Vec<*mut ()>>,
}

impl StateStore {
  pub fn state<T: 'static>(&mut self, f: impl FnOnce(&T)) {
    unsafe { f(self.get_state_raw()) }
  }
  pub fn state_mut<T: 'static>(&mut self, f: impl FnOnce(&mut T)) {
    unsafe { f(self.get_state_raw()) }
  }
  pub fn state_get<T: 'static, R>(&mut self, f: impl FnOnce(&T) -> R) -> R {
    unsafe { f(self.get_state_raw()) }
  }

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
  view: V,
  state: T,
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
  view: V,
  pick: F,
  phantom: PhantomData<(T1, T2)>,
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

pub trait ViewExt: View {
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl View;
  fn with_state_pick<T1: 'static, T2: 'static>(self, len: impl Fn(&mut T1) -> &mut T2)
    -> impl View;
}

impl<T: View> ViewExt for T {
  fn with_local_state_inject<X: 'static>(self, state: X) -> impl View {
    StateCtxInject { view: self, state }
  }
  fn with_state_pick<T1: 'static, T2: 'static>(
    self,
    len: impl Fn(&mut T1) -> &mut T2,
  ) -> impl View {
    StateCtxPick {
      view: self,
      pick: len,
      phantom: PhantomData,
    }
  }
}
