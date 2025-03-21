use crate::*;

pub struct OneToOneRefHashBookKeeping<T: ReactiveQuery> {
  pub upstream: T,
  pub mapping: Arc<RwLock<FastHashMap<T::Value, T::Key>>>,
}

impl<T> ReactiveQuery for OneToOneRefHashBookKeeping<T>
where
  T: ReactiveQuery,
  T::Value: CKey,
{
  type Key = T::Value;
  type Value = T::Key;

  type Compute = impl ReactiveQueryCompute<Key = Self::Key, Value = Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Compute {
    let mapping = self.mapping.make_write_holder();

    OneToOneRefHashBookKeepingCompute {
      upstream: self.upstream.poll_changes(cx),
      mapping: Some(mapping),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.mapping.write().shrink_to_fit(),
    }
    self.upstream.request(request);
  }
}

pub struct OneToOneRefHashBookKeepingCompute<T: ReactiveQueryCompute> {
  pub upstream: T,
  pub mapping: Option<LockWriteGuardHolder<FastHashMap<T::Value, T::Key>>>,
}

impl<T: ReactiveQueryCompute<Value: CKey>> ReactiveQueryCompute
  for OneToOneRefHashBookKeepingCompute<T>
{
  type Key = T::Value;
  type Value = T::Key;
  type Changes = impl Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View = LockReadGuardHolder<FastHashMap<Self::Key, Self::Value>>;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (d, _) = self.upstream.resolve();
    let mut mapping = self.mapping.take().expect("query has already resolved");
    let mut mutations = FastHashMap::<T::Value, ValueChange<T::Key>>::default();
    let mut mutator = QueryMutationCollector {
      delta: &mut mutations,
      target: mapping.deref_mut(),
    };

    for (k, change) in d.iter_key_value() {
      match change {
        ValueChange::Delta(v, pv) => {
          if let Some(pv) = &pv {
            mutator.remove(pv.clone());
          }

          let _check = mutator.set_value(v.clone(), k.clone());
          // todo, optional check the relation is valid one to one
        }
        ValueChange::Remove(pv) => {
          mutator.remove(pv);
        }
      }
    }

    (mutations, mapping.downgrade_to_read())
  }
}
