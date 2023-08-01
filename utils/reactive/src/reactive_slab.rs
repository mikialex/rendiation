// use slab::Slab;

// use crate::*;

// pub struct ReactiveSlab<T> {
//   inner: Slab<(T, channel_single::SingleSender<usize>)>,
//   waker: AtomicWaker,
// }

// struct AllocatedSlot {
//   //
// }

// impl<T> ReactiveSlab<T> {
//   pub fn allocate(&self) -> (impl Stream<Item = usize>, usize) {
//     //
//     self.waker.wake();
//   }
// }

// impl<T> Stream for ReactiveSlab<T> {
//   type Item = ();

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
//     self.inner.compact(|_, from, to| {})
//   }
// }
