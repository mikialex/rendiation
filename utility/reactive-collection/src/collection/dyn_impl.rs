use futures::FutureExt;

use crate::*;

pub type DynReactiveCollectionTask<K, V> = Box<
  dyn Future<
      Output = (
        Box<dyn DynVirtualCollection<K, ValueChange<V>>>,
        Box<dyn DynVirtualCollection<K, V>>,
      ),
    > + Unpin
    + Send,
>;

pub trait DynReactiveCollection<K: CKey, V: CValue>: Sync + Send + 'static {
  fn poll_changes_dyn(&self, cx: &mut Context) -> DynReactiveCollectionTask<K, V>;

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation);
}

impl<K: CKey, V: CValue, T: ReactiveCollection<K, V>> DynReactiveCollection<K, V> for T {
  fn poll_changes_dyn(&self, cx: &mut Context) -> DynReactiveCollectionTask<K, V> {
    Box::new(
      self
        .poll_changes(cx)
        .map(|(d, v)| {
          {
            (
              Box::new(d) as Box<dyn DynVirtualCollection<K, ValueChange<V>>>,
              Box::new(v) as Box<dyn DynVirtualCollection<K, V>>,
            )
          }
        })
        .boxed(),
    )
  }

  fn extra_request_dyn(&mut self, request: &mut ExtraCollectionOperation) {
    self.extra_request(request)
  }
}

impl<K: CKey, V: CValue> ReactiveCollection<K, V> for Box<dyn DynReactiveCollection<K, V>> {
  type Changes = Box<dyn DynVirtualCollection<K, ValueChange<V>>>;
  type View = Box<dyn DynVirtualCollection<K, V>>;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;
  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    self.deref().poll_changes_dyn(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request_dyn(request)
  }
}
