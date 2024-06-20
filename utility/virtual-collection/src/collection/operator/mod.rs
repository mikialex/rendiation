use crate::*;

mod map;
pub use map::*;
mod filter;
pub use filter::*;

pub trait VirtualCollectionExt<K: CKey, V: CValue>:
  VirtualCollection<K, V> + Sized + 'static
{
  fn into_boxed(self) -> Box<dyn VirtualCollection<K, V>> {
    Box::new(self)
  }

  fn map<V2: CValue>(
    self,
    mapper: impl Fn(&K, V) -> V2 + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<K, V2> {
    MappedCollection {
      base: self.into_boxed(),
      mapper,
    }
  }

  fn filter_map<V2: CValue>(
    self,
    mapper: impl Fn(V) -> Option<V2> + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<K, V2> {
    CollectionFilter {
      base: self.into_boxed(),
      mapper,
    }
  }

  fn key_dual_map_partial<K2: CKey>(
    self,
    f1: impl Fn(K) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> Option<K> + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<K2, V> {
    KeyDualMapCollection {
      base: self.into_boxed(),
      f1,
      f2,
    }
  }

  fn key_dual_map<K2: CKey>(
    self,
    f1: impl Fn(K) -> K2 + Clone + Send + Sync + 'static,
    f2: impl Fn(K2) -> K + Clone + Send + Sync + 'static,
  ) -> impl VirtualCollection<K2, V> {
    self.key_dual_map_partial(f1, move |k| Some(f2(k)))
  }
}
impl<T: ?Sized, K: CKey, V: CValue> VirtualCollectionExt<K, V> for T where
  Self: VirtualCollection<K, V> + Sized + 'static
{
}
