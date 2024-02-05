use crate::*;

pub struct ReactiveCollectionDebug<T, K, V> {
  pub inner: T,
  pub state: RwLock<FastHashMap<K, V>>,
  pub label: &'static str,
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionDebug<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    let r = self.inner.poll_changes(cx);

    // validation
    if let Poll::Ready(changes) = &r {
      let changes = changes.materialize();
      let mut state = self.state.write();
      for (k, change) in changes.iter() {
        match change {
          ValueChange::Delta(n, p) => {
            if let Some(removed) = state.remove(k) {
              let p = p.as_ref().expect("previous value should exist");
              assert_eq!(&removed, p);
            } else {
              assert!(p.is_none());
            }
            state.insert(k.clone(), n.clone());
          }
          ValueChange::Remove(p) => {
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

#[derive(Clone)]
pub struct DiffChangedView<'a, K, V> {
  inner: CollectionChanges<'a, K, V>,
}

impl<'a, K, V> VirtualCollection<K, ValueChange<V>> for DiffChangedView<'a, K, V>
where
  K: CKey,
  V: CValue,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, ValueChange<V>)> + '_> {
    Box::new(
      self
        .inner
        .iter_key_value()
        .filter(|(_, v)| !v.is_redundant()),
    )
  }

  fn access(&self, key: &K) -> Option<ValueChange<V>> {
    let change = self.inner.access(key)?;
    if change.is_redundant() {
      None
    } else {
      Some(change)
    }
  }
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionDiff<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue + PartialEq,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    self
      .inner
      .poll_changes(cx)
      .map(|v| Box::new(DiffChangedView { inner: v }) as CollectionChanges<K, V>)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.inner.access()
  }
}

#[pin_project::pin_project]
pub struct ReactiveCollectionAsStream<T, K, V> {
  #[pin]
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> futures::Stream for ReactiveCollectionAsStream<T, K, V>
where
  T: ReactiveCollection<K, V> + Unpin,
  K: CKey,
  V: CValue,
{
  type Item = Arc<FastHashMap<K, ValueChange<V>>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this
      .inner
      .poll_changes(cx)
      .map(|delta| Some(delta.materialize()))
  }
}
