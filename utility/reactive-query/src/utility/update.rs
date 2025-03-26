use crate::*;

struct QueryBasedUpdater<T, F> {
  query: T,
  update_logic: F,
}

pub trait QueryBasedUpdaterExt: ReactiveQuery {
  fn into_collective_updater<TV: Default + CValue>(
    self,
    update_logic: impl FnOnce(Self::Value, &mut TV) + Copy,
  ) -> impl QueryBasedUpdate<Box<dyn QueryLikeMutateTarget<Self::Key, TV>>>;
}

impl<T> QueryBasedUpdaterExt for T
where
  T: ReactiveQuery,
  T::Compute: QueryCompute<Key = T::Key, Value = T::Value>,
{
  fn into_collective_updater<TV: Default + CValue>(
    self,
    update_logic: impl FnOnce(T::Value, &mut TV) + Copy,
  ) -> impl QueryBasedUpdate<Box<dyn QueryLikeMutateTarget<T::Key, TV>>> {
    QueryBasedUpdater {
      query: self,
      update_logic,
    }
  }
}

impl<T, TV, F> QueryBasedUpdate<Box<dyn QueryLikeMutateTarget<T::Key, TV>>>
  for QueryBasedUpdater<T, F>
where
  F: FnOnce(T::Value, &mut TV) + Copy,
  T: ReactiveQuery,
  T::Compute: QueryCompute<Key = T::Key, Value = T::Value>,
  TV: Default + CValue,
{
  fn update_target(
    &mut self,
    target: &mut Box<dyn QueryLikeMutateTarget<T::Key, TV>>,
    cx: &mut Context,
  ) {
    let ((d, _), cx) = self.query.describe(cx).resolve_with_cx();
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

pub trait QueryBasedUpdate<T: ?Sized> {
  fn update_target(&mut self, target: &mut T, cx: &mut Context);
}

/// this struct is to support merge multiple query updates into one target
#[derive(Default)]
pub struct MultiUpdateContainer<T> {
  pub target: T,
  source: Vec<Box<dyn QueryBasedUpdate<T>>>,
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

/// for example if we want merge different query changes into one
pub type MultiUpdateMergeMutation<K, V> =
  MultiUpdateContainer<Box<dyn QueryLikeMutateTarget<K, V>>>;

impl<T> MultiUpdateContainer<T> {
  pub fn new(target: T) -> Self {
    Self {
      target,
      source: Default::default(),
      waker: Default::default(),
    }
  }
  pub fn add_source(&mut self, source: impl QueryBasedUpdate<T> + 'static) {
    self.source.push(Box::new(source));
    self.waker.wake();
  }

  pub fn with_source(mut self, source: impl QueryBasedUpdate<T> + 'static) -> Self {
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

impl<T: 'static> ReactiveGeneralQuery for SharedMultiUpdateContainer<T> {
  type Output = Box<dyn Any>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    self.inner.write().poll_update(cx);
    Box::new(self.inner.make_read_holder())
  }
}
