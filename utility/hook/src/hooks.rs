use std::{
  any::{Any, TypeId},
  io::Write,
  panic::Location,
  sync::Arc,
};

use bumpalo::Bump;
use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;

#[allow(clippy::missing_safety_doc)]
pub unsafe trait HooksCxLike: Sized {
  fn memory_mut(&mut self) -> &mut FunctionMemory;
  fn memory_ref(&self) -> &FunctionMemory;
  fn flush(&mut self);
  fn is_dynamic_stage(&self) -> bool;

  fn is_creating(&self) -> bool {
    !self.memory_ref().created
  }

  fn execute<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
    let r = f(self);
    self.memory_mut().current_cursor = 0;
    self.memory_mut().sub_scope_cursor = 0;

    self.memory_mut().created = true;
    self.flush();
    r
  }

  #[track_caller]
  fn raw_scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
    let is_dynamic_stage = self.is_dynamic_stage();

    let location = FastLocation(Location::caller());
    let key = SubFunctionKeyType::CallSite(location);
    let sub_memory = self.memory_mut().sub_function(is_dynamic_stage, key) as *mut _;

    unsafe {
      core::ptr::swap(self.memory_mut(), sub_memory);
      let r = f(self);
      core::ptr::swap(self.memory_mut(), sub_memory);
      r
    }
  }

  fn raw_keyed_scope<K: std::hash::Hash, R>(
    &mut self,
    key: &K,
    f: impl FnOnce(&mut Self) -> R,
  ) -> R {
    let is_dynamic_stage = self.is_dynamic_stage();

    let key = create_key_from_hash_impl(key);
    let sub_memory = self.memory_mut().sub_function(is_dynamic_stage, key) as *mut _;

    unsafe {
      core::ptr::swap(self.memory_mut(), sub_memory);
      let r = f(self);
      core::ptr::swap(self.memory_mut(), sub_memory);
      r
    }
  }

  #[track_caller]
  fn skip_if_not<R: Default>(&mut self, should_execute: bool, f: impl FnOnce(&mut Self) -> R) -> R {
    let is_dyn = self.is_dynamic_stage();
    let is_creating = self.is_creating();
    let must_execute = is_dyn && is_creating;
    if should_execute || must_execute {
      self.scope(f)
    } else {
      self.skip_call_site_scope();
      R::default()
    }
  }

  #[track_caller]
  fn skip_call_site_scope(&mut self) {
    let key = SubFunctionKeyType::CallSite(FastLocation(Location::caller()));
    let is_dynamic_stage = self.is_dynamic_stage();
    self.memory_mut().sub_function(is_dynamic_stage, key);
  }

  fn skip_keyed_scope<K: std::hash::Hash>(&mut self, key: &K) {
    let key = create_key_from_hash_impl(key);
    let is_dynamic_stage = self.is_dynamic_stage();
    self.memory_mut().sub_function(is_dynamic_stage, key);
  }

  #[track_caller]
  fn scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
    self.raw_scope(|cx| cx.execute(|cx| f(cx)))
  }
  fn keyed_scope<K: std::hash::Hash, R>(&mut self, key: &K, f: impl FnOnce(&mut Self) -> R) -> R {
    self.raw_keyed_scope(key, |cx| cx.execute(|cx| f(cx)))
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T);

  fn use_plain_state_default<T: 'static + Default>(&mut self) -> (&mut Self, &mut T) {
    self.use_plain_state(Default::default)
  }
  fn use_plain_state_default_cloned<T: 'static + Default + Clone>(&mut self) -> (&mut Self, T) {
    let (cx, r) = self.use_plain_state::<T>(Default::default);
    (cx, r.clone())
  }

  fn use_sharable_plain_state<T: 'static>(
    &mut self,
    f: impl FnOnce() -> T,
  ) -> (&mut Self, &mut Arc<RwLock<T>>) {
    self.use_plain_state(|| Arc::new(RwLock::new(f())))
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
  /// (value ptr, drop cx ptr)
  cleanup_fn: fn(*mut (), *mut ()),
  /// (value ptr)
  drop_fn: fn(*mut ()),
}

#[derive(Default)]
pub struct FunctionMemory {
  pub created: bool,
  states: Bump,
  states_meta: Vec<FunctionMemoryState>,
  pub current_cursor: usize,
  pub sub_scope_cursor: usize,
  sub_functions: FastHashMap<SubFunctionKey, Self>,
  sub_functions_next: FastHashMap<SubFunctionKey, Self>,
}

#[derive(Eq, PartialEq, Hash, Debug)]
struct SubFunctionKey {
  position: usize,
  key: SubFunctionKeyType,
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub enum SubFunctionKeyType {
  CallSite(FastLocation),
  UserDefined(smallvec::SmallVec<[u8; 32]>),
}

#[derive(Eq, Debug)]
pub struct FastLocation(pub &'static Location<'static>);

impl PartialEq for FastLocation {
  fn eq(&self, other: &Self) -> bool {
    std::ptr::eq(self.0, other.0)
  }
}
impl std::hash::Hash for FastLocation {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (self.0 as *const _ as usize).hash(state);
  }
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

        fn drop_fn<T>(ptr: &mut T) {
          unsafe {
            core::ptr::drop_in_place(ptr);
          }
        }
        let drop_fn = std::mem::transmute::<fn(&mut T), fn(*mut ())>(drop_fn);

        self.states_meta.push(FunctionMemoryState {
          ptr: init as *mut T as *mut (),
          type_id: TypeId::of::<T>(),
          type_name,
          cleanup_fn,
          drop_fn,
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

  pub fn sub_function(&mut self, is_dynamic_stage: bool, key: SubFunctionKeyType) -> &mut Self {
    let key = SubFunctionKey {
      position: self.sub_scope_cursor,
      key,
    };
    self.sub_scope_cursor += 1;
    if is_dynamic_stage {
      if let Some(previous_memory) = self.sub_functions.remove(&key) {
        assert!(
          !self.sub_functions_next.contains_key(&key),
          "sub function already been used in dynamic stage: {:?}",
          key
        );
        self
          .sub_functions_next
          .entry(key)
          .or_insert(previous_memory)
      } else {
        assert!(
          !self.sub_functions_next.contains_key(&key),
          "sub function already been used in dynamic stage: {:?}",
          key
        );
        self.sub_functions_next.entry(key).or_default()
      }
    } else {
      // todo, validate all function are used
      if let Some(f) = self.sub_functions.get_mut(&key) {
        f
      } else {
        panic!("expect sub function: {:?} not found in static stage", key)
      }
    }
  }

  pub fn flush(&mut self, drop_cx: *mut ()) {
    for (_, mut sub_function) in self.sub_functions.drain() {
      sub_function.cleanup(drop_cx);
    }
    std::mem::swap(&mut self.sub_functions, &mut self.sub_functions_next);
  }

  pub fn cleanup(&mut self, drop_cx: *mut ()) {
    self.states_meta.drain(..).for_each(|meta| {
      (meta.cleanup_fn)(meta.ptr, drop_cx);
      (meta.drop_fn)(meta.ptr);
    });
    self.sub_functions.drain().for_each(|(_, mut f)| {
      f.cleanup(drop_cx);
    })
  }

  // todo, add validation. or we may leak resource
  pub fn flush_assume_only_plain_states(&mut self) {
    for (_, mut sub_function) in self.sub_functions.drain() {
      sub_function.cleanup_assume_only_plain_states();
    }
    std::mem::swap(&mut self.sub_functions, &mut self.sub_functions_next);
  }

  // todo, add validation. or we may leak resource
  pub fn cleanup_assume_only_plain_states(&mut self) {
    self.states_meta.drain(..).for_each(|meta| {
      (meta.drop_fn)(meta.ptr);
    });
    self.sub_functions.drain().for_each(|(_, mut f)| {
      f.cleanup_assume_only_plain_states();
    })
  }
}

fn create_key_from_hash_impl<K: std::hash::Hash>(key: &K) -> SubFunctionKeyType {
  /// this is hack, and has the possibility of hash collision
  /// because the hash impl can hash only part of the data.
  /// todo, improve
  #[derive(Default)]
  struct HashByteCollector(smallvec::SmallVec<[u8; 32]>);
  impl std::hash::Hasher for HashByteCollector {
    fn finish(&self) -> u64 {
      0 // i don't care the hash
    }

    fn write(&mut self, bytes: &[u8]) {
      let r = self.0.write_all(bytes);
      assert!(r.is_ok());
    }
  }

  let mut hasher = HashByteCollector::default();
  key.hash(&mut hasher);
  let key = hasher.0;

  SubFunctionKeyType::UserDefined(key)
}
