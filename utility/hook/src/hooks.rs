use std::{
  any::{Any, TypeId},
  panic::Location,
};

use bumpalo::Bump;
use fast_hash_collection::FastHashMap;

#[allow(clippy::missing_safety_doc)]
pub unsafe trait HooksCxLike: Sized {
  fn memory_mut(&mut self) -> &mut FunctionMemory;
  fn memory_ref(&self) -> &FunctionMemory;
  fn flush(&mut self);

  fn is_creating(&self) -> bool {
    !self.memory_ref().created
  }

  fn execute<R>(&mut self, f: impl FnOnce(&mut Self) -> R, rollback: bool) -> R {
    let start_cursor = self.memory_ref().current_cursor;
    let r = f(self);
    if rollback {
      self.memory_mut().current_cursor = start_cursor;
    }

    self.memory_mut().created = true;
    self.flush();
    r
  }

  #[track_caller]
  fn raw_scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
    let sub_memory = self.memory_mut().sub_function() as *mut _;

    unsafe {
      core::ptr::swap(self.memory_mut(), sub_memory);
      let r = f(self);
      core::ptr::swap(self.memory_mut(), sub_memory);
      r
    }
  }

  #[track_caller]
  fn scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
    self.raw_scope(|cx| cx.execute(|cx| f(cx), true))
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T);

  fn use_plain_state_default<T: 'static + Default>(&mut self) -> (&mut Self, &mut T) {
    self.use_plain_state(Default::default)
  }
  fn use_plain_state_default_cloned<T: 'static + Default + Clone>(&mut self) -> (&mut Self, T) {
    let (cx, r) = self.use_plain_state::<T>(Default::default);
    (cx, r.clone())
  }
}

#[derive(Default)]
pub struct NothingToDrop<T>(pub T);

pub trait CanCleanUpFrom<T> {
  fn drop_from_cx(&mut self, cx: &mut T);
}

impl<T, X: CanCleanUpFrom<T>> CanCleanUpFrom<T> for Option<X> {
  fn drop_from_cx(&mut self, cx: &mut T) {
    if let Some(x) = self {
      x.drop_from_cx(cx);
    }
  }
}

struct FunctionMemoryState {
  ptr: *mut (),
  type_id: TypeId,
  type_name: &'static str,
  cleanup_fn: fn(*mut (), *mut ()),
}

#[derive(Default)]
pub struct FunctionMemory {
  pub created: bool,
  states: Bump,
  states_meta: Vec<FunctionMemoryState>,
  pub current_cursor: usize,
  sub_functions: FastHashMap<Location<'static>, Self>,
  sub_functions_next: FastHashMap<Location<'static>, Self>,
}

impl FunctionMemory {
  pub fn expect_state_init<T: Any, DropCx>(
    &mut self,
    init: impl FnOnce() -> T,
    cleanup: fn(&mut T, &mut DropCx),
  ) -> &mut T {
    unsafe {
      if self.states_meta.len() == self.current_cursor {
        let init = self.states.alloc_with(init);

        let cleanup_fn =
          std::mem::transmute::<fn(&mut T, &mut DropCx), fn(*mut (), *mut ())>(cleanup);

        #[cfg(debug_assertions)]
        let type_name = std::any::type_name::<T>();

        #[cfg(not(debug_assertions))]
        let type_name = "";

        self.states_meta.push(FunctionMemoryState {
          ptr: init as *mut T as *mut (),
          type_id: TypeId::of::<T>(),
          type_name,
          cleanup_fn,
        });
      }
      let FunctionMemoryState {
        type_id,
        ptr,
        #[allow(unused_variables)]
        type_name,
        ..
      } = &mut self.states_meta[self.current_cursor];

      let validate_state_access = true;
      if validate_state_access && *type_id != TypeId::of::<T>() {
        #[cfg(debug_assertions)]
        {
          println!("expect type: {}", std::any::type_name::<T>());
          println!("stored type: {}", type_name);
        }
        panic!("type_miss_match");
      }

      self.current_cursor += 1;
      &mut *(*ptr as *mut T)
    }
  }

  #[track_caller]
  pub fn sub_function(&mut self) -> &mut Self {
    let location = Location::caller();
    if let Some(previous_memory) = self.sub_functions.remove(location) {
      self
        .sub_functions_next
        .entry(*location)
        .or_insert(previous_memory)
    } else {
      self.sub_functions_next.entry(*location).or_default()
    }
  }

  pub fn flush(&mut self, drop_cx: Option<*mut ()>) {
    if let Some(drop_cx) = drop_cx {
      for (_, mut sub_function) in self.sub_functions.drain() {
        sub_function.cleanup(drop_cx);
      }
    } else {
      assert!(
        self.sub_functions.is_empty(),
        "in none flushable stage, sub_functions should be empty"
      );
    }
    std::mem::swap(&mut self.sub_functions, &mut self.sub_functions_next);
    for sub_function in self.sub_functions.values_mut() {
      sub_function.flush(drop_cx);
    }
  }

  pub fn cleanup(&mut self, drop_cx: *mut ()) {
    self.states_meta.drain(..).for_each(|meta| {
      (meta.cleanup_fn)(meta.ptr, drop_cx);
    });
    self.sub_functions.drain().for_each(|(_, mut f)| {
      f.cleanup(drop_cx);
    })
  }
}
