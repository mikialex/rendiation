use crate::*;

pub trait ReactiveQuery {
  type Output;
  fn poll_query(&mut self, cx: &mut Context)
    -> Box<dyn Future<Output = Self::Output> + Unpin + '_>;
}

pub struct ReactiveCollectionAsReactiveQuery<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveQuery for ReactiveCollectionAsReactiveQuery<K, V, T>
where
  K: CKey,
  V: CValue,
  T: ReactiveCollection<K, V>,
{
  type Output = Box<dyn Any>;

  fn poll_query(
    &mut self,
    cx: &mut Context,
  ) -> Box<dyn Future<Output = Self::Output> + Unpin + '_> {
    Box::new(
      self
        .inner
        .poll_changes_dyn(cx)
        .map(|(_, v)| Box::new(v) as Box<dyn Any>),
    )
  }
}

pub struct ReactiveCollectionSelfContainedAsReactiveQuery<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveQuery for ReactiveCollectionSelfContainedAsReactiveQuery<K, V, T>
where
  K: CKey,
  V: CValue,
  T: ReactiveCollectionSelfContained<K, V>,
{
  type Output = Box<dyn Any>;

  fn poll_query(
    &mut self,
    cx: &mut Context,
  ) -> Box<dyn Future<Output = Self::Output> + Unpin + '_> {
    Box::new(
      self
        .inner
        .poll_changes_dyn(cx)
        .map(|(_, v)| Box::new(v) as Box<dyn Any>),
    )
  }
}

pub struct ReactiveManyOneRelationAsReactiveQuery<K, V, T> {
  pub inner: T,
  pub phantom: PhantomData<(K, V)>,
}

impl<K, V, T> ReactiveQuery for ReactiveManyOneRelationAsReactiveQuery<K, V, T>
where
  K: CKey,
  V: CKey,
  T: ReactiveOneToManyRelation<K, V>,
{
  type Output = Box<dyn Any>;

  fn poll_query(
    &mut self,
    cx: &mut Context,
  ) -> Box<dyn Future<Output = Self::Output> + Unpin + '_> {
    Box::new(
      self
        .inner
        .poll_changes_dyn(cx)
        .map(|(_, v)| Box::new(v) as Box<dyn Any>),
    )
  }
}

pub struct ReactiveQueryBoxAnyResult<T>(pub T);

impl<T> ReactiveQuery for ReactiveQueryBoxAnyResult<T>
where
  T::Output: 'static,
  T: ReactiveQuery,
{
  type Output = Box<dyn Any>;

  fn poll_query(
    &mut self,
    cx: &mut Context,
  ) -> Box<dyn Future<Output = Self::Output> + Unpin + '_> {
    let f = self.0.poll_query(cx);
    let f = f.map(|v| Box::new(v) as Box<dyn Any>);
    Box::new(f)
  }
}
