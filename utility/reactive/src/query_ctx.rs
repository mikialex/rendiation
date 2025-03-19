use anymap::AnyMap;

use crate::*;

/// This is a container to hold reactive query or general query like object.
/// Using a container instead of maintain the query at each business logic site is
/// to enable the concurrent execution of the registered queries. This container
/// can be seen as a dynamic sized join operator.
#[derive(Default)]
pub struct ReactiveQueryCtx {
  registry: FastHashMap<u32, (BoxedAnyReactiveQuery, NotifyScope)>,
  next: u32,
}

impl ReactiveQueryCtx {
  pub fn register(&mut self, update: BoxedAnyReactiveQuery) -> QueryToken {
    self
      .registry
      .insert(self.next, (update, Default::default()));
    let token = self.next;
    self.next += 1;
    QueryToken(token)
  }

  pub fn deregister(&mut self, token: &mut QueryToken) {
    self.registry.remove(&std::mem::take(token).0);
  }

  pub fn register_multi_updater<T: 'static>(
    &mut self,
    updater: MultiUpdateContainer<T>,
  ) -> QueryToken {
    let updater = Box::new(SharedMultiUpdateContainer::new(updater)) as BoxedAnyReactiveQuery;
    self.register(updater)
  }

  pub fn register_reactive_query<C>(&mut self, c: C) -> QueryToken
  where
    C: ReactiveQuery + Unpin,
  {
    let c = Box::new(c.into_reactive_state()) as BoxedAnyReactiveQuery;
    self.register(c)
  }

  pub fn register_val_refed_reactive_query<C>(&mut self, c: C) -> QueryToken
  where
    C: ReactiveValueRefQuery,
  {
    let c = Box::new(c.into_reactive_state_self_contained()) as BoxedAnyReactiveQuery;
    self.register(c)
  }

  pub fn register_multi_reactive_query<C>(&mut self, c: C) -> QueryToken
  where
    C: ReactiveOneToManyRelation,
  {
    let c = Box::new(c.into_reactive_state_many_one()) as BoxedAnyReactiveQuery;
    self.register(c)
  }

  pub fn poll_update_all(&mut self, cx: &mut Context) -> QueryResultCtx {
    let mut token_based_waked = FastHashMap::default();
    let token_based_result = self
      .registry
      .iter_mut()
      .map(|(k, v)| {
        let (waked, result) = v
          .1
          .run_and_return_previous_waked(cx, |cx| v.0.poll_query(cx));
        token_based_waked.insert(*k, waked);
        (*k, result)
      })
      .collect();
    QueryResultCtx {
      token_based_result,
      token_based_waked,
      type_based_result: Default::default(),
    }
  }
}

/// A token to identify the registered query
#[derive(Clone, Copy, Debug)]
pub struct QueryToken(u32);

/// We do not impose RAII like ownership on query token. This allows the user to
/// register and deregister/ reregister the query at any time.
///
/// the default value for query token is an invalid empty token.
/// This is by design to facilitate the user to easily derive their default impl
impl Default for QueryToken {
  fn default() -> Self {
    Self(u32::MAX)
  }
}

/// The joined update result of [[ReactiveQueryCtx]], accessed by [[QueryToken]]
pub struct QueryResultCtx {
  pub token_based_result: FastHashMap<u32, Box<dyn Any>>,
  pub token_based_waked: FastHashMap<u32, bool>,
  /// this field provides convenient way to inject any adhoc result for parameter passing
  pub type_based_result: AnyMap,
}

impl QueryResultCtx {
  pub fn take_result(&mut self, token: QueryToken) -> Option<Box<dyn Any>> {
    self.token_based_result.remove(&token.0)
  }
  pub fn check_token_waked(&self, token: QueryToken) -> Option<bool> {
    self.token_based_waked.get(&token.0).copied()
  }

  pub fn take_reactive_query_updated<K: CKey, V: CValue>(
    &mut self,
    token: QueryToken,
  ) -> Option<BoxedDynQuery<K, V>> {
    self
      .take_result(token)?
      .downcast::<BoxedDynQuery<K, V>>()
      .ok()
      .map(|v| *v)
  }

  pub fn take_reactive_multi_query_updated<K: CKey, V: CKey>(
    &mut self,
    token: QueryToken,
  ) -> Option<BoxedDynMultiQuery<K, V>> {
    self
      .take_result(token)?
      .downcast::<BoxedDynMultiQuery<K, V>>()
      .ok()
      .map(|v| *v)
  }

  pub fn take_val_refed_reactive_query_updated<K: CKey, V: CValue>(
    &mut self,
    token: QueryToken,
  ) -> Option<BoxedDynValueRefQuery<K, V>> {
    self
      .take_result(token)?
      .downcast::<BoxedDynValueRefQuery<K, V>>()
      .ok()
      .map(|v| *v)
  }

  pub fn take_multi_updater_updated<T>(
    &mut self,
    token: QueryToken,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    self
      .take_result(token)?
      .downcast::<LockReadGuardHolder<MultiUpdateContainer<T>>>()
      .ok()
      .map(|v| *v)
  }
}

/// This trait abstract the general pattern of the [[ReactiveQueryCtx]] usage
pub trait QueryBasedFeature<T> {
  type Context;
  /// register queries in qcx
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, ctx: &Self::Context);
  /// deregister queries in qcx
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx);
  /// access the ctx's update result, and create the required feature
  fn create_impl(&self, cx: &mut QueryResultCtx) -> T;
}

pub type BoxedAnyReactiveQuery = Box<dyn ReactiveGeneralQuery<Output = Box<dyn Any>>>;
