// use std::sync::Weak;

// use super::{helper::ForkedView, internal::*};
// use crate::*;

// // impl<Map> AsyncQueryCompute for ReactiveQueryForkCompute<Map>
// // where
// //   Map: AsyncQueryCompute,
// // {
// //   type Task = impl Future<Output = (Self::Changes, Self::View)>;

// //   fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
// //     let mut changes = self.changes.clone();
// //     self.view.create_resolve_task(cx).map(move |v| {
// //       let d = changes.resolve();
// //       (d, v)
// //     })
// //   }
// // }

// pub struct AsyncForkComputeView<T: QueryCompute> {
//   upstream: T,
//   downstream: Arc<RwLock<FastHashMap<u64, DownStreamInfo<T::Key, T::Value>>>>,
//   resolved: Arc<RwLock<Option<Weak<dyn Any + Send + Sync>>>>,
// }

// impl<T: AsyncQueryCompute> AsyncForkComputeView<T> {
//   pub fn create_resolve_task(
//     &mut self,
//     cx: &mut AsyncQueryCtx,
//   ) -> impl Future<Output = ForkedView<T::View>> {
//     let downstream = self.downstream.clone();
//     let resolved = self.resolved.clone();
//     let fut = self.upstream.create_task(cx).map(move |upstream| {
//       ForkComputeView {
//         upstream,
//         downstream,
//         resolved,
//       }
//       .resolve()
//     });
//     Box::new(Box::pin(fut))
//   }
// }

// struct FutureForkerInternal<T> {
//   upstream: T,
//   downstream: Vec<usize>,
// }

// enum MaybeResolvedFuture<T: Future> {
//   Resolved(T::Output),
//   Pending(T),
// }

// impl<T: Future> Future for MaybeResolvedFuture<T> {
//   type Output = T::Output;

//   fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//     // match self {
//     //   Self::Resolved(_) => todo!(),
//     //   Self::Pending(_) => todo!(),
//     // }
//     todo!()
//   }
// }

// struct FutureForker<T> {
//   internal: Arc<RwLock<FutureForkerInternal<T>>>,
// }

// impl<T: Future> Future for FutureForker<T> {
//   type Output = T::Output;

//   fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//     todo!()
//   }
// }

// impl<T> FutureForker<T> {
//   pub fn fork(&self) -> Self {
//     let internal = self.internal.clone();
//     FutureForker { internal }
//   }
// }
