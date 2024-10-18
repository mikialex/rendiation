use crate::*;

mod map;
pub use map::*;
mod filter;
pub use filter::*;

pub trait QueryExt: Query + Sized + 'static {
  fn into_boxed(self) -> BoxedDynQuery<Self::Key, Self::Value> {
    Box::new(self)
  }

  fn map<V2: CValue>(
    self,
    mapper: impl Fn(&Self::Key, Self::Value) -> V2 + Clone + Send + Sync + 'static,
  ) -> impl Query<Key = Self::Key, Value = V2> {
    MappedQuery { base: self, mapper }
  }

  fn filter_map<V2: CValue>(
    self,
    mapper: impl Fn(Self::Value) -> Option<V2> + Clone + Send + Sync + 'static,
  ) -> impl Query<Key = Self::Key, Value = V2> {
    FilterQuery { base: self, mapper }
  }

  fn key_dual_map_partial<K2: CKey>(
    self,
    f1: impl Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Option<Self::Key> + Clone + Send + Sync + 'static,
  ) -> impl Query<Key = K2, Value = Self::Value> {
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
