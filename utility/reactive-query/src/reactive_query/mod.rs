use crate::*;

mod self_contain;
pub use self_contain::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

pub enum ReactiveQueryRequest {
  MemoryShrinkToFit,
}

pub trait ReactiveQuery: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  type Compute: AsyncQueryCompute<Key = Self::Key, Value = Self::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute;

  fn request(&mut self, request: &mut ReactiveQueryRequest);
}

#[derive(Clone, Default)]
pub struct QueryResolveCtx {
  kept_view: Arc<RwLock<Vec<Box<dyn Any + Send + Sync>>>>,
}

impl QueryResolveCtx {
  pub fn keep_view_alive(&self, view: impl Any + Send + Sync) {
    self.kept_view.write().push(Box::new(view));
  }
}

pub trait QueryCompute: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  type Changes: Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View: Query<Key = Self::Key, Value = Self::Value> + 'static;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View);

  fn resolve_kept(&mut self) -> (Self::Changes, KeptQuery<Self::View>) {
    let cx = Default::default();
    let r = self.resolve(&cx);
    (r.0, r.1.keep_sth(cx))
  }
}

pub struct AsyncQueryCtx {
  resolve_cx: QueryResolveCtx,
}

pub struct AsyncQuerySpawner;
impl AsyncQuerySpawner {
  pub fn spawn_task<R>(
    &self,
    f: impl FnOnce() -> R + 'static,
  ) -> impl Future<Output = R> + 'static {
    // todo, use some thread pool impl
    async move { f() }
  }
}

impl AsyncQueryCtx {
  pub fn resolve_cx(&self) -> &QueryResolveCtx {
    &self.resolve_cx
  }
  pub fn make_spawner(&self) -> AsyncQuerySpawner {
    AsyncQuerySpawner
  }
  pub fn spawn_task<R>(
    &self,
    f: impl FnOnce() -> R + 'static,
  ) -> impl Future<Output = R> + 'static {
    self.make_spawner().spawn_task(f)
  }

  #[inline(always)]
  pub fn then_spawn<T: 'static, R>(
    &self,
    f: impl Future<Output = T> + 'static,
    then: impl FnOnce(T, &QueryResolveCtx) -> R + 'static,
  ) -> impl Future<Output = R> + 'static {
    let sp = self.make_spawner();
    let cx = self.resolve_cx.clone();
    f.then(move |s| sp.spawn_task(move || then(s, &cx)))
  }

  #[inline(always)]
  pub fn then_spawn_compute<T: 'static, R: QueryCompute>(
    &self,
    f: impl Future<Output = T> + 'static,
    then: impl FnOnce(T) -> R + 'static,
  ) -> impl Future<Output = (R::Changes, R::View)> + 'static {
    self.then_spawn(f, |inner, cx| {
      let mut r = then(inner);
      r.resolve(cx)
    })
  }
}

pub trait AsyncQueryCompute: QueryCompute {
  // this is correct version
  type Task: Future<Output = (Self::Changes, Self::View)> + Send + Sync + 'static;
  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task;
}

impl<K, V, Change, View> QueryCompute for (Change, View)
where
  K: CKey,
  V: CValue,
  Change: Query<Key = K, Value = ValueChange<V>> + 'static,
  View: Query<Key = K, Value = V> + 'static,
{
  type Key = K;
  type Value = V;
  type Changes = Change;
  type View = View;
  fn resolve(&mut self, _cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    (self.0.clone(), self.1.clone())
  }
}
impl<K, V, Change, View> AsyncQueryCompute for (Change, View)
where
  K: CKey,
  V: CValue,
  Change: Query<Key = K, Value = ValueChange<V>> + 'static,
  View: Query<Key = K, Value = V> + 'static,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)> + 'static;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    futures::future::ready(self.resolve(cx.resolve_cx()))
  }
}

impl<K: CKey, V: CValue> ReactiveQuery for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;
  type Compute = (EmptyQuery<K, ValueChange<V>>, EmptyQuery<K, V>);
  fn describe(&self, _: &mut Context) -> Self::Compute {
    (Default::default(), Default::default())
  }
  fn request(&mut self, _: &mut ReactiveQueryRequest) {}
}
