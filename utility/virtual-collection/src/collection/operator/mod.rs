use crate::*;

mod map;
pub use map::*;
mod filter;
pub use filter::*;

pub trait VirtualCollectionExt: VirtualCollection + Sized + 'static {
  fn into_boxed(self) -> BoxedDynVirtualCollection<Self::Key, Self::Value> {
    Box::new(self)
  }

  fn map<V2: CValue>(
    self,
    mapper: impl Fn(&Self::Key, Self::Value) -> V2 + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<Key = Self::Key, Value = V2> {
    MappedCollection { base: self, mapper }
  }

  fn filter_map<V2: CValue>(
    self,
    mapper: impl Fn(Self::Value) -> Option<V2> + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<Key = Self::Key, Value = V2> {
    CollectionFilter { base: self, mapper }
  }

  fn key_dual_map_partial<K2: CKey>(
    self,
    f1: impl Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Option<Self::Key> + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<Key = K2, Value = Self::Value> {
    KeyDualMapCollection { base: self, f1, f2 }
  }

  fn key_dual_map<K2: CKey>(
    self,
    f1: impl Fn(Self::Key) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Self::Key + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<Key = K2, Value = Self::Value> {
    self.key_dual_map_partial(f1, move |k| Some(f2(k)))
  }
}
impl<T: ?Sized> VirtualCollectionExt for T where Self: VirtualCollection + Sized + 'static {}
