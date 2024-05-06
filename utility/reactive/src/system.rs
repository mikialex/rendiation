use std::any::{Any, TypeId};

use fast_hash_collection::FastHashMap;

use crate::*;

pub type BoxedFutureStream = Box<dyn Stream<Item = BoxedAnyFuture>>;
pub type BoxedAnyFuture = Box<dyn Future<Output = Box<dyn Any>>>;

/// ordered update logic, will be updated in order
///
/// this will be to top level structure to contain frame logic for any application
pub struct OrderedStreamContainer {
  update_logic: Vec<Box<dyn Stream<Item = ()>>>,
}

impl OrderedStreamContainer {
  pub fn poll_update(&mut self, cx: &mut Context) {
    //
  }
}

pub struct ConcurrentStreamContainer {
  update_logic: FastHashMap<u32, BoxedFutureStream>,
  next: u32,
}

impl ConcurrentStreamContainer {
  pub fn register(&mut self, update: BoxedFutureStream) -> UpdateResultToken {
    todo!()
  }

  pub fn register_multi_updater<T: 'static>(
    &mut self,
    updater: MultiUpdateContainer<T>,
  ) -> UpdateResultToken {
    // let updater = Box::new(SharedMultiUpdateContainer::new(updater)) as BoxedFutureStream;
    // self.register(TypeId::of::<MultiUpdateContainer<T>>(), updater);
    todo!()
  }

  pub fn register_reactive_collection<C, K, V>(&mut self, c: C) -> UpdateResultToken
  where
    K: CKey,
    V: CValue,
    C: ReactiveCollection<K, V>,
  {
    // let c = Box::new(c);
    // let c = todo!();
    // self.register_source_raw(TypeId::of::<C>(), c);
    todo!()
  }

  pub fn register_self_contained_reactive_collection<C, K, V>(&mut self, c: C) -> UpdateResultToken
  where
    K: CKey,
    V: CValue,
    C: ReactiveCollectionSelfContained<K, V>,
  {
    // let c = Box::new(c);
    // let c = todo!();
    // self.register_source_raw(TypeId::of::<C>(), c);
    todo!()
  }

  pub fn register_reactive_multi_collection<C, K, V>(&mut self, c: C) -> UpdateResultToken
  where
    C: ReactiveOneToManyRelationship<K, V>,
    K: CKey,
    V: CKey,
  {
    // let c = Box::new(c);
    // let c = todo!();
    // self.register_source_raw(TypeId::of::<C>(), c);
    todo!()
  }

  pub fn poll_update_all(&self, cx: &mut Context) -> ConcurrentStreamUpdateResult {
    // loop
    // join_all(
    //   self
    //     .get_mut()
    //     .resource
    //     .values_mut()
    //     .map(|v| v.poll_next(cx)),
    // );
    todo!()
  }
}

#[derive(Clone, Copy)]
pub struct UpdateResultToken(u32);

impl Default for UpdateResultToken {
  fn default() -> Self {
    Self(u32::MAX)
  }
}

pub struct ConcurrentStreamUpdateResult {
  inner: FastHashMap<u32, Box<dyn Any>>,
}

impl ConcurrentStreamUpdateResult {
  pub fn get_result(&self, token: UpdateResultToken) -> Option<Box<dyn Any>> {
    todo!()
  }

  pub fn get_reactive_collection_updated<K, V>(
    &self,
    token: UpdateResultToken,
  ) -> Option<Box<dyn VirtualCollection<K, V>>> {
    todo!()
  }

  pub fn get_multi_reactive_collection_updated<K, V>(
    &self,
    token: UpdateResultToken,
  ) -> Option<Box<dyn VirtualMultiCollection<K, V>>> {
    todo!()
  }
  pub fn get_self_contained_reactive_collection_updated<K, V>(
    &self,
    token: UpdateResultToken,
  ) -> Option<Box<dyn VirtualCollectionSelfContained<K, V>>> {
    todo!()
  }

  pub fn get_multi_updater<T>(
    &self,
    token: UpdateResultToken,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    // let t = TypeId::of::<MultiUpdateContainer<T>>();
    // self
    //   .inner
    //   .get(&t)?
    //   .downcast_ref::<LockReadGuardHolder<MultiUpdateContainer<T>>>()?
    //   .clone()
    //   .into()

    todo!()
  }
}
