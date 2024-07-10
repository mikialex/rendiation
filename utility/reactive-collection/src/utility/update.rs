use futures::future::join_all;

use crate::*;

pub struct CollectionDeltaUpdateLogic<K, V, F> {
  pub delta: Box<dyn DynVirtualCollection<K, ValueChange<V>>>,
  pub update_logic: F,
}

impl<K, V, TV, F> CollectionUpdate<Box<dyn CollectionLikeMutateTarget<K, TV>>>
  for CollectionDeltaUpdateLogic<K, V, F>
where
  F: FnOnce(V, &mut TV) + Copy,
  K: CKey,
  V: CValue,
  TV: Default + CValue,
{
  fn update_target(&mut self, target: &mut Box<dyn CollectionLikeMutateTarget<K, TV>>) {
    for (k, v) in self.delta.iter_key_value() {
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
  fn update_target(&mut self, target: &mut T);
}

/// this struct is to support merge multiple collection updates into one target
#[derive(Default)]
pub struct MultiUpdateContainer<T> {
  pub target: T,
  source: Vec<Box<dyn Stream<Item = Box<dyn CollectionUpdate<T>>> + Unpin>>,
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
  pub fn add_source(
    &mut self,
    source: impl Stream<Item = Box<dyn CollectionUpdate<T>>> + Unpin + 'static,
  ) {
    self.source.push(Box::new(source));
    self.waker.wake();
  }

  pub fn with_source(
    mut self,
    source: impl Stream<Item = Box<dyn CollectionUpdate<T>>> + Unpin + 'static,
  ) -> Self {
    self.source.push(Box::new(source));
    self.waker.wake();
    self
  }
}

impl<T> MultiUpdateContainer<T> {
  pub fn poll_update(&mut self, cx: &mut Context) -> impl Future<Output = ()> + '_ {
    self.waker.register(cx.waker());

    join_all(self.source.iter_mut().map(|s| s.next())).map(|updates| {
      updates
        .into_iter()
        .flatten()
        .for_each(|mut update| update.update_target(&mut self.target))
    })
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

  fn poll_query(
    &mut self,
    cx: &mut Context,
  ) -> Box<dyn Future<Output = Self::Output> + Unpin + '_> {
    let inner = self.inner.clone();
    let mut i = self.inner.write();
    let task = i.poll_update(cx);
    let task = Box::new(task) as Box<dyn Future<Output = ()> + Unpin>;

    // we known that's safe
    let task: Box<dyn Future<Output = ()> + Unpin + 'static> =
      unsafe { std::mem::transmute::<_, _>(task) };
    Box::new(Box::pin(async move {
      task.await;
      Box::new(inner.make_read_holder()) as Box<dyn Any>
    }))
  }
}
