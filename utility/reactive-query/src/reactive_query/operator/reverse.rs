use crate::*;

pub struct OneToOneRefHashBookKeeping<T, K: CKey, V: CValue> {
  pub upstream: T,
  pub mapping: Arc<RwLock<FastHashMap<V, K>>>,
}

impl<T> ReactiveQuery for OneToOneRefHashBookKeeping<T, T::Key, T::Value>
where
  T: ReactiveQuery,
  T::Value: CKey,
{
  type Key = T::Value;
  type Value = T::Key;

  type Compute = OneToOneRefHashBookKeeping<T::Compute, T::Key, T::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    OneToOneRefHashBookKeeping {
      upstream: self.upstream.describe(cx),
      mapping: self.mapping.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.mapping.write().shrink_to_fit(),
    }
    self.upstream.request(request);
  }
}

impl<T: QueryCompute<Value: CKey>> QueryCompute
  for OneToOneRefHashBookKeeping<T, T::Key, T::Value>
{
  type Key = T::Value;
  type Value = T::Key;
  type Changes = Arc<FastHashMap<Self::Key, ValueChange<Self::Value>>>;
  type View = LockReadGuardHolder<FastHashMap<Self::Key, Self::Value>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, _) = self.upstream.resolve(cx);
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

    (Arc::new(mutations), self.mapping.make_read_holder())
  }
}

impl<T: AsyncQueryCompute<Value: CKey>> AsyncQueryCompute
  for OneToOneRefHashBookKeeping<T, T::Key, T::Value>
{
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mapping = self.mapping.clone();
    let upstream = self.upstream.create_task(cx);

    cx.then_spawn(upstream, move |upstream, cx| {
      OneToOneRefHashBookKeeping { upstream, mapping }.resolve(cx)
    })
  }
}
