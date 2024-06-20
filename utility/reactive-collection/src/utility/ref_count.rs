use crate::*;

#[derive(Clone)]
pub struct CollectionSetsRefcount<T, K> {
  source_sets: Arc<RwLock<Vec<Box<dyn ReactiveCollection<T, K>>>>>,
  wake_for_new_source: Arc<AtomicWaker>,
  ref_count: Arc<RwLock<FastHashMap<K, u32>>>,
}

impl<T, K> Default for CollectionSetsRefcount<T, K> {
  fn default() -> Self {
    Self {
      source_sets: Default::default(),
      wake_for_new_source: Default::default(),
      ref_count: Default::default(),
    }
  }
}

impl<T, K> CollectionSetsRefcount<T, K> {
  pub fn add_source(&self, source: Box<dyn ReactiveCollection<T, K>>) {
    self.source_sets.write().push(source);
    self.wake_for_new_source.wake();
  }
}

impl<T: CKey, K: CKey> ReactiveCollection<K, u32> for CollectionSetsRefcount<T, K> {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, u32> {
    self.wake_for_new_source.register(cx.waker());

    let mut ref_count = self.ref_count.write();
    let sources = self.source_sets.read();

    let mut mutations = FastHashMap::<K, ValueChange<u32>>::default();
    let mut mutator = CollectionMutationCollector {
      delta: &mut mutations,
      target: ref_count.deref_mut(),
    };

    for source in sources.iter() {
      if let Poll::Ready(deltas) = source.poll_changes(cx) {
        for (_, delta) in deltas.iter_key_value() {
          match delta {
            ValueChange::Delta(k, pk) => {
              if pk.is_none() {
                if let Some(pre_rc) = mutator.remove(k.clone()) {
                  mutator.set_value(k.clone(), pre_rc + 1);
                } else {
                  mutator.set_value(k.clone(), 1);
                }
              }
            }
            ValueChange::Remove(k) => {
              let pre_rc = mutator.remove(k.clone()).unwrap();
              if pre_rc - 1 > 0 {
                mutator.set_value(k.clone(), pre_rc - 1);
              }
            }
          }
        }
      }
    }

    if mutations.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Box::new(mutations))
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, u32> {
    Box::new(self.ref_count.make_read_holder())
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    let mut sources = self.source_sets.write();
    for source in sources.iter_mut() {
      source.extra_request(request);
    }
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.ref_count.write().shrink_to_fit(),
    }
  }
}
