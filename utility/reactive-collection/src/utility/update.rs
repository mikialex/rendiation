use crate::*;

struct CollectionUpdater<T, V, F> {
  phantom: PhantomData<V>,
  collection: T,
  update_logic: F,
}

pub trait CollectionUpdaterExt<K, V> {
  fn into_collective_updater<TV: Default + CValue>(
    self,
    update_logic: impl FnOnce(V, &mut TV) + Copy,
  ) -> impl CollectionUpdate<Box<dyn CollectionLikeMutateTarget<K, TV>>>;
}

impl<T, K, V> CollectionUpdaterExt<K, V> for T
where
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn into_collective_updater<TV: Default + CValue>(
    self,
    update_logic: impl FnOnce(V, &mut TV) + Copy,
  ) -> impl CollectionUpdate<Box<dyn CollectionLikeMutateTarget<K, TV>>> {
    CollectionUpdater {
      phantom: PhantomData,
      collection: self,
      update_logic,
    }
  }
}

impl<T, K, V, TV, F> CollectionUpdate<Box<dyn CollectionLikeMutateTarget<K, TV>>>
  for CollectionUpdater<T, V, F>
where
  F: FnOnce(V, &mut TV) + Copy,
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
  TV: Default + CValue,
{
  fn update_target(
    &mut self,
    target: &mut Box<dyn CollectionLikeMutateTarget<K, TV>>,
    cx: &mut Context,
  ) {
    let (d, _) = self.collection.poll_changes(cx);
    for (k, v) in d.iter_key_value() {
      match v {
        ValueChange::Delta(v, _) => {
          if target.get_current(k.clone()).is_none() {
            target.set_value(k.clone(), Default::default());
          }
          target.mutate(k, &|t| (self.update_logic)(v.clone(), t));
        }
        ValueChange::Remove(_) => {
          target.remove(k);
        }
      }
    }
  }
}

pub trait CollectionUpdate<T: ?Sized> {
  fn update_target(&mut self, target: &mut T, cx: &mut Context);
  fn flush(&mut self) {}
}

/// this struct is to support merge multiple collection updates into one target
#[derive(Default)]
pub struct MultiUpdateContainer<T> {
  pub target: T,
  source: Vec<Box<dyn CollectionUpdate<T>>>,
  waker: Arc<AtomicWaker>,
}

impl<T> Deref for MultiUpdateContainer<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.target
  }
}
impl<T> DerefMut for MultiUpdateContainer<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.target
  }
}

/// for example if we want merge different collection changes into one
pub type MultiUpdateMergeMutation<K, V> =
  MultiUpdateContainer<Box<dyn CollectionLikeMutateTarget<K, V>>>;

impl<T> MultiUpdateContainer<T> {
  pub fn new(target: T) -> Self {
    Self {
      target,
      source: Default::default(),
      waker: Default::default(),
    }
  }
  pub fn add_source(&mut self, source: impl CollectionUpdate<T> + 'static) {
    self.source.push(Box::new(source));
    self.waker.wake();
  }

  pub fn with_source(mut self, source: impl CollectionUpdate<T> + 'static) -> Self {
    self.source.push(Box::new(source));
    self.waker.wake();
    self
  }
}

impl<T> MultiUpdateContainer<T> {
  pub fn poll_update(&mut self, cx: &mut Context) {
    self.waker.register(cx.waker());
    for source in &mut self.source {
      source.update_target(&mut self.target, cx)
    }
  }
}

pub struct SharedMultiUpdateContainer<T> {
  inner: Arc<RwLock<MultiUpdateContainer<T>>>,
}

impl<T> SharedMultiUpdateContainer<T> {
  pub fn new(inner: MultiUpdateContainer<T>) -> Self {
    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }
}

impl<T: 'static> ReactiveQuery for SharedMultiUpdateContainer<T> {
  type Output = Box<dyn Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    self.inner.write().poll_update(cx);
    Box::new(self.inner.make_read_holder())
  }
}
