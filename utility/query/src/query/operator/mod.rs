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

  fn map<V2, F>(self, mapper: F) -> MappedQuery<F, Self>
  where
    F: Fn(&Self::Key, Self::Value) -> V2,
  {
    MappedQuery { base: self, mapper }
  }

  fn filter_map<V2, F>(self, mapper: F) -> FilterMapQuery<F, Self>
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

  fn key_dual_map<K2: CKey>(
    self,
    f1: impl Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Self::Key + Clone + Send + Sync + 'static,
  ) -> impl Query<Key = K2, Value = Self::Value> {
    self.key_dual_map_partial(f1, move |k| Some(f2(k)))
  }
}
impl<T: ?Sized> QueryExt for T where Self: Query + Sized + 'static {}
