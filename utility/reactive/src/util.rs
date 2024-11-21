use crate::*;

pub type BoxedAnyReactiveQuery = Box<dyn ReactiveGeneralQuery<Output = Box<dyn Any>>>;

#[derive(Default)]
pub struct ReactiveQueryJoinUpdater {
  update_logic: FastHashMap<u32, BoxedAnyReactiveQuery>,
  next: u32,
}

impl ReactiveQueryJoinUpdater {
  pub fn register(&mut self, update: BoxedAnyReactiveQuery) -> UpdateResultToken {
    self.update_logic.insert(self.next, update);
    let token = self.next;
    self.next += 1;
    UpdateResultToken(token)
  }

  pub fn deregister(&mut self, token: &mut UpdateResultToken) {
    self.update_logic.remove(&std::mem::take(token).0);
  }

  pub fn register_multi_updater<T: 'static>(
    &mut self,
    updater: MultiUpdateContainer<T>,
  ) -> UpdateResultToken {
    let updater = Box::new(SharedMultiUpdateContainer::new(updater)) as BoxedAnyReactiveQuery;
    self.register(updater)
  }

  pub fn register_reactive_query<C>(&mut self, c: C) -> UpdateResultToken
  where
    C: ReactiveQuery + Unpin,
  {
    let c = Box::new(c.into_reactive_state()) as BoxedAnyReactiveQuery;
    self.register(c)
  }

  pub fn register_val_refed_reactive_query<C>(&mut self, c: C) -> UpdateResultToken
  where
    C: ReactiveValueRefQuery,
  {
    let c = Box::new(c.into_reactive_state_self_contained()) as BoxedAnyReactiveQuery;
    self.register(c)
  }

  pub fn register_multi_reactive_query<C>(&mut self, c: C) -> UpdateResultToken
  where
    C: ReactiveOneToManyRelation,
  {
    let c = Box::new(c.into_reactive_state_many_one()) as BoxedAnyReactiveQuery;
    self.register(c)
  }

  pub fn poll_update_all(&mut self, cx: &mut Context) -> ConcurrentStreamUpdateResult {
    ConcurrentStreamUpdateResult {
      inner: self
        .update_logic
        .iter_mut()
        .map(|(k, v)| (*k, v.poll_query(cx)))
        .collect(),
    }
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
  pub fn take_result(&mut self, token: UpdateResultToken) -> Option<Box<dyn Any>> {
    self.inner.remove(&token.0)
  }

  pub fn take_reactive_query_updated<K: CKey, V: CValue>(
    &mut self,
    token: UpdateResultToken,
  ) -> Option<BoxedDynQuery<K, V>> {
    self
      .take_result(token)?
      .downcast::<BoxedDynQuery<K, V>>()
      .ok()
      .map(|v| *v)
  }

  pub fn take_reactive_multi_query_updated<K: CKey, V: CKey>(
    &mut self,
    token: UpdateResultToken,
  ) -> Option<BoxedDynMultiQuery<K, V>> {
    self
      .take_result(token)?
      .downcast::<BoxedDynMultiQuery<K, V>>()
      .ok()
      .map(|v| *v)
  }
  pub fn take_val_refed_reactive_query_updated<K: CKey, V: CValue>(
    &mut self,
    token: UpdateResultToken,
  ) -> Option<BoxedDynValueRefQuery<K, V>> {
    self
      .take_result(token)?
      .downcast::<BoxedDynValueRefQuery<K, V>>()
      .ok()
      .map(|v| *v)
  }

  pub fn take_multi_updater_updated<T>(
    &mut self,
    token: UpdateResultToken,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    self
      .take_result(token)?
      .downcast::<LockReadGuardHolder<MultiUpdateContainer<T>>>()
      .ok()
      .map(|v| *v)
  }
}
