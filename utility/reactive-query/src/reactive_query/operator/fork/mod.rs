mod helper;
mod internal;

use helper::*;
use internal::*;

use crate::*;

pub struct ReactiveQueryFork<Map: ReactiveQuery> {
  internal: ReactiveKVMapForkInternal<Map>,
  fork: DownStreamFork<Map::Key, Map::Value>,
}

impl<Map: ReactiveQuery> ReactiveQueryFork<Map> {
  #[track_caller]
  pub fn new(upstream: Map, as_static_forker: bool) -> Self {
    let internal = ReactiveKVMapForkInternal::new(upstream);
    let fork = internal.create_child(as_static_forker);

    ReactiveQueryFork { internal, fork }
  }

  pub fn is_static_forker(&self) -> bool {
    self.fork.as_static_forker
  }
}

impl<Map: ReactiveQuery> Drop for ReactiveQueryFork<Map> {
  fn drop(&mut self) {
    self.internal.remove_child(&self.fork);
  }
}

impl<Map> Clone for ReactiveQueryFork<Map>
where
  Map: ReactiveQuery,
{
  fn clone(&self) -> Self {
    self.clone_impl(false)
  }
}

impl<Map> ReactiveQueryFork<Map>
where
  Map: ReactiveQuery,
{
  #[track_caller]
  pub fn clone_as_static(&self) -> Self {
    self.clone_impl(true)
  }

  #[track_caller]
  fn clone_impl(&self, as_static_forker: bool) -> Self {
    let fork = self.internal.create_child(as_static_forker);
    Self {
      internal: self.internal.clone(),
      fork,
    }
  }
}

impl<Map> ReactiveQuery for ReactiveQueryFork<Map>
where
  Map: ReactiveQuery,
{
  type Key = Map::Key;
  type Value = Map::Value;
  type Compute = ReactiveQueryForkCompute<Map>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    // install new waker, this waker is shared by arc within the downstream info
    self.fork.update_waker(cx);

    ReactiveQueryForkCompute {
      view: self.internal.poll_and_broadcast(),
      changes: self.fork.drain_changes(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.internal.request(request)
  }
}

pub struct ReactiveQueryForkCompute<M: ReactiveQuery> {
  view: ForkComputeView<M>,
  changes: DrainChange<M::Key, M::Value>,
}

impl<Map> QueryCompute for ReactiveQueryForkCompute<Map>
where
  Map: ReactiveQuery,
{
  type Key = Map::Key;
  type Value = Map::Value;

  type Changes = BoxedDynQuery<Map::Key, ValueChange<Map::Value>>;
  type View = ForkedView<<Map::Compute as QueryCompute>::View>;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let view = self.view.resolve();
    let changes = self.changes.resolve(); // this must been called after view resolve.
    (changes, view)
  }
}

impl<K: CKey, V: CValue> RQForker<K, V> {
  pub fn update_and_read(&self) -> BoxedDynQuery<K, V> {
    self.internal.poll_and_broadcast().resolve().into_boxed()
  }
}

impl<K: CKey, V: CKey> OneManyRelationForker<K, V> {
  pub fn update_and_read(&self) -> BoxedDynMultiQuery<K, V> {
    self
      .internal
      .poll_and_broadcast()
      .resolve()
      .into_boxed_multi()
  }
}
