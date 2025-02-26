#![feature(downcast_unchecked)]

use std::any::{Any, TypeId};

use fast_hash_collection::*;
// for downstream crates use utils macro
pub use once_cell;
use parking_lot::*;
pub use paste;

pub struct DowncasterRegistry<T: ?Sized> {
  downcaster: RwLock<FastHashMap<TypeId, (fn(&dyn Any) -> &T, fn(&mut dyn Any) -> &mut T)>>,
}

impl<T: ?Sized> Default for DowncasterRegistry<T> {
  fn default() -> Self {
    Self {
      downcaster: Default::default(),
    }
  }
}

impl<T: ?Sized> DowncasterRegistry<T> {
  pub fn register<X: AsRef<T> + AsMut<T> + 'static>(&self) {
    // we could use unchecked downcast here because we could assure the
    // item passed in at downcast method always matches the real type
    self.downcaster.write().insert(
      TypeId::of::<X>(),
      (
        |item| unsafe {
          let typed = item.downcast_ref_unchecked::<X>();
          typed.as_ref()
        },
        |item| unsafe {
          let typed = item.downcast_mut_unchecked::<X>();
          typed.as_mut()
        },
      ),
    );
  }

  pub fn downcast_ref<'a>(&self, item: &'a dyn Any) -> Option<&'a T> {
    self
      .downcaster
      .read()
      .get(&Any::type_id(item))
      .map(|(f, _)| f(item))
  }
  pub fn downcast_ref_unwrap<'a>(&self, item: &'a dyn Any) -> &'a T {
    self
      .downcast_ref(item)
      .expect("item must registered before")
  }

  pub fn downcast_mut<'a>(&self, item: &'a mut dyn Any) -> Option<&'a mut T> {
    self
      .downcaster
      .read()
      .get(&Any::type_id(item))
      .map(|(_, f)| f(item))
  }

  pub fn downcast_mut_unwrap<'a>(&self, item: &'a mut dyn Any) -> &'a mut T {
    self
      .downcast_mut(item)
      .expect("item must registered before")
  }
}

#[macro_export]
macro_rules! type_as_dyn_trait {
  ($Type: ty, $Trait:ident) => {
    impl AsRef<dyn $Trait> for $Type {
      fn as_ref(&self) -> &(dyn $Trait + 'static) {
        self
      }
    }

    impl AsMut<dyn $Trait> for $Type {
      fn as_mut(&mut self) -> &mut (dyn $Trait + 'static) {
        self
      }
    }
  };
}

#[macro_export]
macro_rules! define_dyn_trait_downcaster_static {
    ($Trait:ident) => {
        paste::paste! {
            #[allow(non_upper_case_globals)]
            pub static [< Dyn_trait_registry_ $Trait >]: once_cell::sync::Lazy<DowncasterRegistry<dyn $Trait>> =
            once_cell::sync::Lazy::new(Default::default);
        }
    };
}

#[macro_export]
macro_rules! get_dyn_trait_downcaster_static {
  ($Trait:ident) => {
    paste::paste! { &[< Dyn_trait_registry_ $Trait >] }
  };
}

#[test]
fn downcaster() {
  struct Test {
    a: usize,
  }

  define_dyn_trait_downcaster_static!(TestTrait);
  pub trait TestTrait {
    fn size(&self) -> usize;
    fn add(&mut self);
  }

  type_as_dyn_trait!(Test, TestTrait);
  impl TestTrait for Test {
    fn size(&self) -> usize {
      self.a
    }
    fn add(&mut self) {
      self.a += 1;
    }
  }

  let registry = get_dyn_trait_downcaster_static!(TestTrait);
  registry.register::<Test>();

  let mut boxed = Box::new(Test { a: 1 }) as Box<dyn Any + 'static>;
  let downcasted = registry.downcast_ref_unwrap(boxed.as_ref());
  assert_eq!(downcasted.size(), 1);

  let downcasted = registry.downcast_mut_unwrap(boxed.as_mut());
  downcasted.add();
  assert_eq!(downcasted.size(), 2);
}
