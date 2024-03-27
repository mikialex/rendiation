use futures::task::AtomicWaker;

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
  ) -> impl CollectionUpdate<K, TV>;
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
  ) -> impl CollectionUpdate<K, TV> {
    CollectionUpdater {
      phantom: PhantomData,
      collection: self,
      update_logic,
    }
  }
}

impl<T, K, V, TV, F> CollectionUpdate<K, TV> for CollectionUpdater<T, V, F>
where
  F: FnOnce(V, &mut TV) + Copy,
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
  TV: Default + CValue,
{
  fn update_target(&mut self, target: &mut dyn MutableCollection<K, TV>, cx: &mut Context) {
    if let Poll::Ready(changes) = self.collection.poll_changes(cx) {
      for (k, v) in changes.iter_key_value() {
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
}

pub trait CollectionUpdate<K, V> {
  fn update_target(&mut self, target: &mut dyn MutableCollection<K, V>, cx: &mut Context);
}

pub struct MultiUpdateSource<T, K, V>
where
  T: MutableCollection<K, V>,
  V: CValue,
{
  target: T,
  source: Vec<Box<dyn CollectionUpdate<K, V>>>,
  waker: Arc<AtomicWaker>,
}

impl<T, K, V> MultiUpdateSource<T, K, V>
where
  T: MutableCollection<K, V>,
  V: CValue,
{
  pub fn poll_update(&mut self, cx: &mut Context) {
    self.waker.register(cx.waker());
    for source in &mut self.source {
      source.update_target(&mut self.target, cx)
    }
  }

  pub fn add_source(&mut self, source: Box<dyn CollectionUpdate<K, V>>) {
    self.source.push(source);
    self.waker.wake();
  }
}
