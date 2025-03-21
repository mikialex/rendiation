use crate::*;

mod map;
pub use map::*;
mod filter;
pub use filter::*;
mod join;
pub use join::*;
mod union;
pub use union::*;

pub trait QueryExt: Query + Sized + 'static {
  fn into_boxed(self) -> BoxedDynQuery<Self::Key, Self::Value> {
    Box::new(self)
  }

  fn map<V2, F>(self, mapper: F) -> MappedQuery<Self, F>
  where
    F: Fn(&Self::Key, Self::Value) -> V2,
  {
    MappedQuery { base: self, mapper }
  }

  fn filter_map<V2, F>(self, mapper: F) -> FilterMapQuery<Self, F>
  where
    F: Fn(Self::Value) -> Option<V2>,
  {
    FilterMapQuery { base: self, mapper }
  }

  fn key_dual_map_partial<K2, F1, F2>(self, f1: F1, f2: F2) -> KeyDualMappedQuery<F1, F2, Self>
  where
    F1: Fn(Self::Key) -> K2,
    F2: Fn(K2) -> Option<Self::Key>,
  {
    KeyDualMappedQuery { base: self, f1, f2 }
  }

  fn key_dual_map<K2, F1, F2>(
    self,
    f1: F1,
    f2: F2,
  ) -> KeyDualMappedQuery<F1, AutoSomeFnResult<F2>, Self>
  where
    K2: CKey,
    F1: Fn(Self::Key) -> K2,
    F2: Fn(K2) -> Self::Key,
  {
    self.key_dual_map_partial(f1, AutoSomeFnResult(f2))
  }
}
impl<T: ?Sized> QueryExt for T where Self: Query + Sized + 'static {}

#[derive(Clone, Copy)]
pub struct AutoSomeFnResult<F>(F);
impl<K, K2, F: FnOnce(K) -> K2> FnOnce<(K,)> for AutoSomeFnResult<F> {
  type Output = Option<K2>;

  extern "rust-call" fn call_once(self, args: (K,)) -> Self::Output {
    Some(self.0(args.0))
  }
}
impl<K, K2, F: FnMut(K) -> K2> FnMut<(K,)> for AutoSomeFnResult<F> {
  extern "rust-call" fn call_mut(&mut self, args: (K,)) -> Self::Output {
    self.call_once(args)
  }
}
impl<K, K2, F: Fn(K) -> K2> Fn<(K,)> for AutoSomeFnResult<F> {
  extern "rust-call" fn call(&self, args: (K,)) -> Self::Output {
    self.call_once(args)
  }
}
