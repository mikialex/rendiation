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
  type Changes = impl Query<Key = Self::Key, Value = ValueChange<Self::Value>>;
  type View = impl Query<Key = Self::Key, Value = Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, _) = self.upstream.poll_changes(cx);

    let mut mapping = self.mapping.write();

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

    drop(mapping);

    (mutations, self.mapping.make_read_holder())
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.mapping.write().shrink_to_fit(),
    }
    self.upstream.request(request);
  }
}
