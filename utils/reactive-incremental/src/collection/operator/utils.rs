use crate::*;

pub struct ReactiveCollectionDebug<T, K, V> {
  pub inner: T,
  pub state: RwLock<FastHashMap<K, V>>,
  pub label: &'static str,
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionDebug<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: std::fmt::Debug + CKey,
  V: std::fmt::Debug + CValue + PartialEq,
{
  fn poll_changes(&self, cx: &mut Context<'_>) -> PollCollectionChanges<K, V> {
    let r = self.inner.poll_changes(cx);

    // validation
    if let CPoll::Ready(Poll::Ready(changes)) = &r {
      let changes = changes.materialize();
      let mut state = self.state.write();
      for (k, change) in changes.iter() {
        match change {
          CollectionDelta::Delta(_, n, p) => {
            if let Some(removed) = state.remove(k) {
              let p = p.as_ref().expect("previous value should exist");
              assert_eq!(&removed, p);
            } else {
              assert!(p.is_some());
            }
            state.insert(k.clone(), n.clone());
          }
          CollectionDelta::Remove(_, p) => {
            let removed = state.remove(k).expect("remove none exist value");
            assert_eq!(&removed, p);
          }
        }
      }
    }

    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.inner.access()
  }
}

pub struct ReactiveCollectionDiff<T, K, V> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionDiff<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: Clone + Send + Sync + Eq + Hash + 'static,
  V: Clone + Send + Sync + PartialEq + 'static,
{
  fn poll_changes(&self, cx: &mut Context<'_>) -> PollCollectionChanges<K, V> {
    let mut is_empty = false;
    let r = self.inner.poll_changes(cx).map(|r| {
      r.map(|v| {
        let map = v.materialize_hashmap_maybe_cloned();
        let map = map
          .into_iter()
          .filter(|(_, v)| match v {
            CollectionDelta::Delta(_, n, Some(p)) => n != p,
            _ => true,
          })
          .collect::<FastHashMap<_, _>>();

        if map.is_empty() {
          is_empty = true;
        }

        Box::new(Arc::new(map)) as Box<dyn VirtualCollection<K, CollectionDelta<K, V>>>
      })
    });

    if is_empty {
      return CPoll::Ready(Poll::Pending);
    }

    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.inner.access()
  }
}
