use crate::*;

#[derive(Clone, Copy)]
pub struct ValueChangeMapper<F>(pub F);
impl<K, V, V2, F: Fn(&K, V) -> V2 + Clone> FnOnce<(&K, ValueChange<V>)> for ValueChangeMapper<F> {
  type Output = ValueChange<V2>;

  extern "rust-call" fn call_once(self, args: (&K, ValueChange<V>)) -> Self::Output {
    self.call(args)
  }
}

impl<K, V, V2, F: Fn(&K, V) -> V2 + Clone> FnMut<(&K, ValueChange<V>)> for ValueChangeMapper<F> {
  extern "rust-call" fn call_mut(&mut self, args: (&K, ValueChange<V>)) -> Self::Output {
    self.call(args)
  }
}

impl<K, V, V2, F: Fn(&K, V) -> V2 + Clone> Fn<(&K, ValueChange<V>)> for ValueChangeMapper<F> {
  extern "rust-call" fn call(&self, (k, v): (&K, ValueChange<V>)) -> Self::Output {
    v.map(|v| (self.0.clone())(k, v))
  }
}

#[derive(Clone, Copy)]
pub struct ValueChangeMapperValueOnly<F>(pub F);
impl<V, V2, F: Fn(V) -> V2 + Clone> FnOnce<(ValueChange<V>,)> for ValueChangeMapperValueOnly<F> {
  type Output = ValueChange<V2>;

  extern "rust-call" fn call_once(self, args: (ValueChange<V>,)) -> Self::Output {
    self.call(args)
  }
}

impl<V, V2, F: Fn(V) -> V2 + Clone> FnMut<(ValueChange<V>,)> for ValueChangeMapperValueOnly<F> {
  extern "rust-call" fn call_mut(&mut self, args: (ValueChange<V>,)) -> Self::Output {
    self.call(args)
  }
}

impl<V, V2, F: Fn(V) -> V2 + Clone> Fn<(ValueChange<V>,)> for ValueChangeMapperValueOnly<F> {
  extern "rust-call" fn call(&self, v: (ValueChange<V>,)) -> Self::Output {
    v.0.map(|v| (self.0.clone())(v))
  }
}

impl<T, U> DualQuery<T, U> {
  pub fn map<K, V, V2, F>(
    self,
    f: F,
  ) -> DualQuery<MappedQuery<T, F>, MappedQuery<U, ValueChangeMapper<F>>>
  where
    K: CKey,
    V: CValue,
    V2: CValue,
    T: Query<Key = K, Value = V>,
    U: Query<Key = K, Value = ValueChange<V>>,
    F: Fn(&K, V) -> V2 + Clone + Sync + Send + 'static,
  {
    DualQuery {
      view: self.view.map(f.clone()),
      delta: self.delta.delta_map(f),
    }
  }
}

#[test]
fn test_delta_map_value() {
  let mut base = FastHashMap::default();
  base.insert(1u32, ValueChange::Delta(10i32, Some(5)));
  base.insert(2, ValueChange::Remove(20));

  let mapped = base.delta_map_value(|v: i32| v * 2);

  validate_query_consistency(&mapped);
  assert_eq!(mapped.access(&1), Some(ValueChange::Delta(20, Some(10))));
  assert_eq!(mapped.access(&2), Some(ValueChange::Remove(40)));
}

#[test]
fn test_delta_map_with_key() {
  let mut base = FastHashMap::default();
  base.insert(1u32, ValueChange::Delta(10i32, Some(5)));

  let mapped = base.delta_map(|k: &u32, v: i32| *k as i32 + v);

  validate_query_consistency(&mapped);
  assert_eq!(mapped.access(&1), Some(ValueChange::Delta(11, Some(6))));
}
