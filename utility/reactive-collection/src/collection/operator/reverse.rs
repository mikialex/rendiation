use crate::*;

pub struct OneToOneRefHashBookKeeping<K, V, T> {
  pub upstream: T,
  pub mapping: Arc<RwLock<FastHashMap<V, K>>>,
}

impl<K, V, T> ReactiveCollection<V, K> for OneToOneRefHashBookKeeping<K, V, T>
where
  K: CKey,
  V: CKey,
  T: ReactiveCollection<K, V>,
{
  type Changes = impl VirtualCollection<V, ValueChange<K>>;
  type View = impl VirtualCollection<V, K>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, _) = self.upstream.poll_changes(cx);

    let mut mapping = self.mapping.write();

    let mut mutations = FastHashMap::<V, ValueChange<K>>::default();
    let mut mutator = CollectionMutationCollector {
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

    (mutations, self.mapping.make_read_holder())
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.mapping.write().shrink_to_fit(),
    }
    self.upstream.extra_request(request);
  }
}
