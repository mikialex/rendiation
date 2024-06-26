use crate::*;

pub struct ReactiveCollectionDebug<T, K, V> {
  pub inner: T,
  pub state: Arc<RwLock<FastHashMap<K, V>>>,
  pub label: &'static str,
}

impl<T, K, V> ReactiveCollection<K, V> for ReactiveCollectionDebug<T, K, V>
where
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  type Changes = T::Changes;
  type View = T::View;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let f = self.inner.poll_changes(cx);
    let s = self.state.clone();

    async move {
      let (d, v) = f.await;

      // validation
      let changes = d.materialize();
      let mut state = s.write();
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

      (d, v)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

pub struct ReactiveCollectionDiff<T, K, V> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

#[derive(Clone)]
pub struct DiffChangedView<T> {
  inner: T,
}

impl<T, K, V> VirtualCollection<K, ValueChange<V>> for DiffChangedView<T>
where
  T: VirtualCollection<K, ValueChange<V>>,
  K: CKey,
  V: CValue,
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K, ValueChange<V>)> + '_ {
    self
      .inner
      .iter_key_value()
      .filter(|(_, v)| !v.is_redundant())
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
  type Changes = impl VirtualCollection<K, ValueChange<V>>;
  type View = impl VirtualCollection<K, V>;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let f = self.inner.poll_changes(cx);
    async {
      let (d, v) = f.await;
      let d = DiffChangedView { inner: d };
      (d, v)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request);
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
    let r = this.inner.poll_changes(cx).0.materialize();

    if r.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(r))
    }
  }
}
