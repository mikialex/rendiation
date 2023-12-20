use crate::*;

pub struct ReactiveKVSubSet<T1, T2, K, V> {
  pub big: BufferedCollection<T1, K, V>,
  pub sub: BufferedCollection<T2, K, ()>,
  pub phantom: PhantomData<(K, V)>,
}

impl<T1, T2, K, V> ReactiveCollection for ReactiveKVSubSet<T1, T2, K, V> {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    todo!()
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    todo!()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    todo!()
  }
}
