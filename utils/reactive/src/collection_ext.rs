use crate::*;

pub trait AllocIdCollectionExt<K: 'static, X> {
  fn collective_execute_simple_map<V>(
    self,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollection<AllocIdx<K>, V>
  where
    V: CValue;
}

impl<K, T, X> AllocIdCollectionExt<K, X> for T
where
  T: ReactiveCollection<AllocIdx<K>, X>,
  K: IncrementalBase,
  X: CValue,
{
  fn collective_execute_simple_map<V>(
    self,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollection<AllocIdx<K>, V>
  where
    V: CValue,
  {
    self.collective_execute_map_by(move || {
      let creator = storage_of::<K>().create_key_mapper(move |m, _| mapper(m));
      move |k, _| creator(*k)
    })
  }
}
