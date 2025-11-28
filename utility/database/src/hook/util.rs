use crate::*;

// this trait could be lift into upper stream
pub trait QueryProvider<K, V>: Send + Sync {
  fn access(&self) -> BoxedDynQuery<K, V>;
}

impl<T: Query + 'static> QueryProvider<T::Key, T::Value> for Arc<RwLock<T>> {
  fn access(&self) -> BoxedDynQuery<T::Key, T::Value> {
    Arc::new(self.make_read_holder())
  }
}

pub type DBViewUnchecked<V> = IterableComponentReadView<V>;
pub type DBView<V> = IterableComponentReadViewChecked<V>;
pub type DBDelta<V> = FastIterQuery<V>;
pub type DBDualQuery<V> = DualQuery<DBView<V>, DBDelta<V>>;
pub type DBSetDualQuery = DualQuery<BoxedDynQuery<RawEntityHandle, ()>, DBDelta<()>>;

pub fn get_db_view_no_generation_check<C: ComponentSemantic>() -> DBViewUnchecked<C::Data> {
  get_db_view_no_generation_check_internal(C::Entity::entity_id(), C::component_id())
}

pub fn get_db_view_no_generation_check_internal<T>(
  e_id: EntityId,
  c_id: ComponentId,
) -> DBViewUnchecked<T> {
  global_database()
    .access_ecg_dyn(e_id, |ecg| {
      ecg.access_component(c_id, |c| IterableComponentReadView {
        ecg: ecg.clone(),
        read_view: c.read_untyped(),
        phantom: PhantomData,
      })
    })
    .unwrap()
}

pub fn get_db_view_internal<T>(e_id: EntityId, c_id: ComponentId) -> DBView<T> {
  global_database()
    .access_ecg_dyn(e_id, |ecg| {
      ecg.access_component(c_id, |c| IterableComponentReadViewChecked {
        ecg: ecg.clone(),
        read_view: c.read_untyped(),
        phantom: PhantomData,
      })
    })
    .unwrap()
}

pub fn get_db_view<C: ComponentSemantic>() -> DBView<C::Data> {
  get_db_view_internal(C::Entity::entity_id(), C::component_id())
}

pub fn get_db_view_uncheck_access<C: ComponentSemantic>() -> IterableComponentReadView<C::Data> {
  global_database()
    .access_ecg_dyn(C::Entity::entity_id(), |ecg| {
      ecg.access_component(C::component_id(), |c| IterableComponentReadView {
        ecg: ecg.clone(),
        read_view: c.read_untyped(),
        phantom: PhantomData,
      })
    })
    .unwrap()
}

pub fn get_db_view_typed<C: ComponentSemantic>(
) -> impl Query<Key = EntityHandle<C::Entity>, Value = C::Data> {
  get_db_view_internal(C::Entity::entity_id(), C::component_id()).mark_entity_type::<C::Entity>()
}

pub fn get_db_view_typed_foreign<C: ForeignKeySemantic>(
) -> impl Query<Key = EntityHandle<C::Entity>, Value = EntityHandle<C::ForeignEntity>> {
  get_db_view_typed::<C>()
    .filter_map(|v| v.map(|v| unsafe { EntityHandle::<C::ForeignEntity>::from_raw(v) }))
}

pub trait RawEntityHandleQueryExt: Query<Key = RawEntityHandle> {
  fn mark_entity_type<E: EntitySemantic>(
    self,
  ) -> impl Query<Key = EntityHandle<E>, Value = Self::Value> {
    self.key_dual_map(|k| unsafe { EntityHandle::<E>::from_raw(k) }, |k| k.handle)
  }
}

impl<T> RawEntityHandleQueryExt for T where T: Query<Key = RawEntityHandle> {}

pub trait RawEntityHandleQueryMultiExt: MultiQuery<Key = RawEntityHandle> + 'static {
  fn mark_entity_type_multi<E: EntitySemantic>(
    self,
  ) -> impl MultiQuery<Key = EntityHandle<E>, Value = Self::Value> {
    self.multi_key_dual_map(|k| unsafe { EntityHandle::<E>::from_raw(k) }, |k| k.handle)
  }

  fn mark_foreign_key<C: ForeignKeySemantic>(
    self,
  ) -> impl MultiQuery<Key = EntityHandle<C::ForeignEntity>, Value = EntityHandle<C::Entity>>
  where
    Self: MultiQuery<Value = RawEntityHandle>,
  {
    self
      .mark_entity_type_multi::<C::ForeignEntity>()
      .multi_map(|k| unsafe { EntityHandle::<C::Entity>::from_raw(k) })
  }
}

impl<T> RawEntityHandleQueryMultiExt for T where T: MultiQuery<Key = RawEntityHandle> + 'static {}

pub type RevRefOfForeignKey<S> = BoxedDynMultiQuery<
  EntityHandle<<S as ForeignKeySemantic>::ForeignEntity>,
  EntityHandle<<S as EntityAssociateSemantic>::Entity>,
>;

#[derive(Clone)]
pub(crate) struct ArenaAccessProvider<T: CValue>(pub(crate) Arc<RwLock<Arena<T>>>);
impl<T: CValue> QueryProvider<RawEntityHandle, T> for ArenaAccessProvider<T> {
  fn access(&self) -> BoxedDynQuery<RawEntityHandle, T> {
    Arc::new(ArenaAccess(self.0.make_read_holder()))
  }
}

#[derive(Clone)]
struct ArenaAccess<T: CValue>(LockReadGuardHolder<Arena<T>>);

impl<V: CValue> Query for ArenaAccess<V> {
  type Key = RawEntityHandle;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (RawEntityHandle, V)> + '_ {
    self.0.iter().map(|(h, v)| {
      let raw = h.into_raw_parts();
      (
        RawEntityHandle(Handle::from_raw_parts(raw.0, raw.1)),
        v.clone(),
      )
    })
  }

  fn access(&self, key: &RawEntityHandle) -> Option<V> {
    let handle = self.0.get_handle(key.index() as usize).unwrap();
    self.0.get(handle).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.0.is_empty()
  }
}

pub(crate) struct ComponentAccess<T> {
  pub(crate) ecg: EntityComponentGroup,
  pub(crate) original: ComponentCollectionUntyped,
  pub(crate) phantom: PhantomData<T>,
}

impl<T: CValue> QueryProvider<u32, T> for ComponentAccess<T> {
  fn access(&self) -> BoxedDynQuery<u32, T> {
    IterableComponentReadView::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.read_untyped(),
      phantom: PhantomData,
    }
    .into_boxed()
  }
}

impl<T: CValue> QueryProvider<RawEntityHandle, T> for ComponentAccess<T> {
  fn access(&self) -> BoxedDynQuery<RawEntityHandle, T> {
    IterableComponentReadViewChecked::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.read_untyped(),
      phantom: PhantomData,
    }
    .into_boxed()
  }
}

pub trait SkipGenerationCheckExt: Sized {
  fn skip_generation_check<E: EntitySemantic>(self) -> SkipGenerationCheck<Self>;
}

impl<T: Query> SkipGenerationCheckExt for T {
  fn skip_generation_check<E: EntitySemantic>(self) -> SkipGenerationCheck<T> {
    SkipGenerationCheck {
      alloc: global_database()
        .access_ecg_dyn(E::entity_id(), |ecg| ecg.inner.allocator.make_read_holder()),
      inner: self,
    }
  }
}

#[derive(Clone)]
pub struct SkipGenerationCheck<T> {
  alloc: LockReadGuardHolder<Arena<()>>,
  inner: T,
}

impl<T: Query<Key = RawEntityHandle>> Query for SkipGenerationCheck<T> {
  type Key = u32;
  type Value = T::Value;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.inner.iter_key_value().map(|(k, v)| {
      let handle = self.alloc.get_handle(k.index() as usize).unwrap();
      (handle.index() as u32, v)
    })
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    let handle = self.alloc.get_handle(*key as usize)?;
    self.inner.access(&RawEntityHandle(handle))
  }

  fn has_item_hint(&self) -> bool {
    self.inner.has_item_hint()
  }
}
