use std::sync::Weak;

use futures::future::ready;

use super::{helper::ForkedView, internal::*};
use crate::*;

impl<Map: AsyncQueryCompute> AsyncQueryCompute for ReactiveQueryForkCompute<Map> {
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mut changes = self.changes.clone();
    self.view.create_upstream_view_future(cx).map(move |v| {
      let d = changes.resolve();
      (d, v)
    })
  }
}

impl<Map: AsyncQueryCompute> ForkComputeView<Map> {
  pub fn create_upstream_view_future(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> impl Future<Output = ForkedView<Map::View>> + 'static {
    let mut future_forker = self.future_forker.write();
    let future_forker: &mut Option<Weak<dyn Any + Send + Sync>> = &mut future_forker;
    if let Some(v) = future_forker {
      if let Some(future) = v.upgrade() {
        let future = future
          .downcast::<DynFutureForker<ForkedView<Map::View>>>()
          .unwrap();
        return future.fork();
      }
    }

    let future = if let Some(view) = &self._already_resolved_view {
      let future = ready(ForkedView {
        inner: view.clone(),
      });
      Box::new(Box::pin(future))
        as Box<dyn Unpin + Send + Sync + Future<Output = ForkedView<Map::View>>>
    } else {
      let downstream = self.downstream.clone();
      let _already_resolved_view = self._already_resolved_view.clone();
      let view_resolve = self.view_resolve.clone();
      let c = cx.resolve_cx().clone();
      let future = self
        .compute
        .as_ref()
        .unwrap()
        .write()
        .create_task(cx)
        .map(move |upstream| {
          ForkComputeView {
            compute: Arc::new(RwLock::new(upstream)).into(),
            downstream,
            _already_resolved_view,
            future_forker: Default::default(),
            view_resolve,
          }
          .resolve(&c)
        });
      Box::new(Box::pin(future))
        as Box<dyn Unpin + Send + Sync + Future<Output = ForkedView<Map::View>>>
    };

    let future = FutureForker::init(future);
    let future_return = future.fork();

    let future = Arc::new(future) as Arc<dyn Any + Send + Sync>;
    *future_forker = Some(Arc::downgrade(&future));

    future_return
  }
}

struct FutureForkerInternal<T: Future> {
  upstream: Option<T>,
  resolve: Option<T::Output>,
}

#[pin_project::pin_project]
pub struct FutureForker<T: Future> {
  internal: Arc<RwLock<FutureForkerInternal<T>>>,
}

impl<T: Future<Output: Clone> + Unpin> Future for FutureForker<T> {
  type Output = T::Output;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let proj = self.project();
    let mut internal = proj.internal.write();
    if let Some(f) = &mut internal.upstream {
      if let Poll::Ready(v) = f.poll_unpin(cx) {
        internal.resolve = Some(v.clone());
        Poll::Ready(v)
      } else {
        Poll::Pending
      }
    } else {
      Poll::Ready(internal.resolve.clone().unwrap())
    }
  }
}

impl<T: Future<Output: Send + Sync + Clone>> FutureForker<T> {
  pub fn init(upstream: T) -> Self {
    let internal = Arc::new(RwLock::new(FutureForkerInternal {
      upstream: Some(upstream),
      resolve: None,
    }));
    FutureForker { internal }
  }
  pub fn fork(&self) -> Self {
    let internal = self.internal.clone();
    FutureForker { internal }
  }
}

pub type DynFutureForker<View> = FutureForker<Box<dyn Unpin + Send + Sync + Future<Output = View>>>;
