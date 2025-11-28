// use crate::*;

// type MutationData<K, T> = FastHashMap<K, ValueChange<T>>;

// pub fn collective_channel<K, T>() -> (
//   CollectiveMutationSender<K, T>,
//   CollectiveMutationReceiver<K, T>,
// ) {
//   let inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)> = Default::default();
//   let sender = CollectiveMutationSender {
//     inner: inner.clone(),
//   };
//   let receiver = CollectiveMutationReceiver { inner };

//   (sender, receiver)
// }

// pub struct CollectiveMutationSender<K, T> {
//   inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)>,
// }

// impl<K, T> Clone for CollectiveMutationSender<K, T> {
//   fn clone(&self) -> Self {
//     Self {
//       inner: self.inner.clone(),
//     }
//   }
// }

// use parking_lot::lock_api::RawRwLock;
// impl<K: CKey, T: CValue> CollectiveMutationSender<K, T> {
//   /// # Safety
//   ///
//   /// this should be called before send
//   pub unsafe fn lock(&self) {
//     self.inner.0.raw().lock_exclusive()
//   }
//   /// # Safety
//   ///
//   /// this should be called after send
//   pub unsafe fn unlock(&self) {
//     let mutations = &mut *self.inner.0.data_ptr();
//     if !mutations.is_empty() {
//       self.inner.1.wake();
//     }
//     self.inner.0.raw().unlock_exclusive()
//   }
//   /// # Safety
//   ///
//   /// this should be called when locked
//   pub unsafe fn send(&self, idx: K, change: ValueChange<T>) {
//     let mutations = &mut *self.inner.0.data_ptr();
//     merge_change(mutations, (idx, change));
//   }
//   /// # Safety
//   ///
//   /// this should be called when locked
//   pub unsafe fn reserve_space(&self, size: usize) {
//     let mutations = &mut *self.inner.0.data_ptr();
//     mutations.reserve(size);
//   }

//   pub fn is_closed(&self) -> bool {
//     // self inner is shared between sender and receiver, if not shared anymore it must be
//     // receiver not exist anymore, so the channel is closed.
//     Arc::strong_count(&self.inner) == 1
//   }
// }

// /// this is not likely to be triggered because component type is not get removed in any time
// impl<K, T> Drop for CollectiveMutationSender<K, T> {
//   fn drop(&mut self) {
//     self.inner.1.wake()
//   }
// }

// pub struct CollectiveMutationReceiver<K, T> {
//   inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)>,
// }

// impl<K: CKey, T: CValue> CollectiveMutationReceiver<K, T> {
//   pub fn poll_impl(&self, cx: &mut Context) -> Poll<Option<MutationData<K, T>>> {
//     self.inner.1.register(cx.waker());
//     let mut changes = self.inner.0.write();
//     let changes: &mut MutationData<K, T> = &mut changes;

//     let changes = std::mem::take(changes);
//     if !changes.is_empty() {
//       Poll::Ready(Some(changes))
//       // check if the sender has been dropped
//     } else if Arc::strong_count(&self.inner) == 1 {
//       Poll::Ready(None)
//     } else {
//       Poll::Pending
//     }
//   }
//   pub fn has_change(&self) -> bool {
//     let changes = self.inner.0.read();
//     !changes.is_empty()
//   }
// }

// impl<K: CKey, T: CValue> Stream for CollectiveMutationReceiver<K, T> {
//   type Item = MutationData<K, T>;

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
//     self.poll_impl(cx)
//   }
// }

// pub(crate) fn add_listen<T: CValue>(
//   query: impl QueryProvider<RawEntityHandle, T>,
//   source: &EventSource<ChangePtr>,
// ) -> CollectiveMutationReceiver<RawEntityHandle, T> {
//   let (sender, receiver) = collective_channel::<RawEntityHandle, T>();
//   // expand initial value while first listen.
//   unsafe {
//     sender.lock();
//     let query = query.access();
//     let iter = query.iter_key_value();

//     let count_hint = iter.size_hint().0;
//     sender.reserve_space(count_hint);

//     for (idx, v) in iter {
//       sender.send(idx, ValueChange::Delta(v, None));
//     }
//     sender.unlock();
//   }

//   source.on(move |change| unsafe {
//     match change {
//       ScopedMessage::Start => {
//         sender.lock();
//         false
//       }
//       ScopedMessage::End => {
//         sender.unlock();
//         sender.is_closed()
//       }
//       ScopedMessage::ReserveSpace(size) => {
//         sender.reserve_space(*size);
//         false
//       }
//       ScopedMessage::Message(write) => {
//         let change = write.change.map(|v| (*(v.0 as *const T)).clone());
//         sender.send(write.idx, change);
//         false
//       }
//     }
//   });
//   receiver
// }
