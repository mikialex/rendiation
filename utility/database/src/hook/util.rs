use crate::*;

pub type DBView<V> = IterableComponentReadViewChecked<V>;
pub type DBDelta<V> = Arc<FastHashMap<RawEntityHandle, ValueChange<V>>>;
pub type DBDualQuery<V> = DualQuery<DBView<V>, DBDelta<V>>;
pub type DBSetDualQuery = DualQuery<BoxedDynQuery<RawEntityHandle, ()>, DBDelta<()>>;

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
