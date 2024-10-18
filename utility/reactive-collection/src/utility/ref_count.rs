use crate::*;

#[derive(Clone)]
pub struct CollectionSetsRefcount<T, K> {
  source_sets: Arc<RwLock<Vec<BoxedDynReactiveCollection<T, K>>>>,
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
  pub fn add_source(&self, source: BoxedDynReactiveCollection<T, K>) {
    self.source_sets.write().push(source);
    self.wake_for_new_source.wake();
  }
}

impl<T: CKey, K: CKey> ReactiveCollection for CollectionSetsRefcount<T, K> {
  type Key = K;
  type Value = u32;
  type Changes = impl VirtualCollection<Key = K, Value = ValueChange<u32>>;
  type View = impl VirtualCollection<Key = K, Value = u32>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    self.wake_for_new_source.register(cx.waker());

    let mut ref_count = self.ref_count.write();
    let sources = self.source_sets.read();

    let mut mutations = FastHashMap::<K, ValueChange<u32>>::default();
    let mut mutator = CollectionMutationCollector {
      delta: &mut mutations,
      target: ref_count.deref_mut(),
    };

    for source in sources.iter() {
      let (d, _) = source.poll_changes(cx);
      for (_, delta) in d.iter_key_value() {
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

    let d = Arc::new(mutations);
    let v = self.ref_count.make_read_holder();
    (d, v)
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
