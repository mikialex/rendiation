use crate::*;

pub struct DBLinearChangeWatchGroup {
  internal: DBChangeWatchGroup<ComponentId>,
}

impl DBLinearChangeWatchGroup {
  pub fn new(db: &Database) -> Self {
    Self {
      internal: DBChangeWatchGroup::new(db),
    }
  }

  pub fn clear_changes(&mut self) {
    self.internal.clear_changes();
  }

  pub fn allocate_next_consumer_id(&mut self) -> u32 {
    self.internal.allocate_next_consumer_id()
  }

  pub fn notify_consumer_dropped(&mut self, component_id: ComponentId, consumer_id: u32) {
    self
      .internal
      .notify_consumer_dropped(component_id, consumer_id);
  }

  pub fn get_buffered_changes<C: ComponentSemantic>(
    &mut self,
    id: u32,
  ) -> Arc<LinearBatchChanges<u32, C::Data>> {
    self.get_buffered_changes_internal(id, C::Entity::entity_id(), C::component_id())
  }

  #[inline(never)] // remove the variant of component semantic to reduce the binary bloat
  pub fn get_buffered_changes_internal<T: CValue>(
    &mut self,
    id: u32,
    e_id: EntityId,
    c_id: ComponentId,
  ) -> Arc<LinearBatchChanges<u32, T>> {
    let rev = self.internal.producers.entry(c_id).or_insert_with(|| {
      let rev = self.internal.db.access_ecg_dyn(e_id, move |e| {
        e.access_component(c_id, move |c| {
          add_listen(
            ComponentAccess {
              ecg: e.clone(),
              original: c.clone(),
              phantom: PhantomData::<T>,
            },
            &c.data_watchers,
          )
        })
        .unwrap()
      });
      Box::new(rev)
    });

    let rev = rev
      .downcast_mut::<CollectiveMutationReceiver<RawEntityHandle, T>>()
      .unwrap();

    let consumer_ids = self.internal.consumers.entry(c_id).or_default();

    let changes = self
      .internal
      .current_results
      .entry(c_id)
      .or_insert_with(|| {
        noop_ctx!(cx);

        // todo, use special listen avoid this cost
        let changes = if let Poll::Ready(Some(r)) = rev.poll_impl(cx) {
          let removed = r
            .iter()
            .filter_map(|v| v.1.is_removed().then_some(v.0.index()))
            .collect::<Vec<_>>();

          let update_or_insert = r
            .iter()
            .filter_map(|v| v.1.new_value().map(|x| (v.0.index(), x.clone())))
            .collect::<Vec<_>>();

          LinearBatchChanges {
            removed,
            update_or_insert,
          }
        } else {
          Default::default()
        };
        Box::new(Arc::new(changes))
      });

    if consumer_ids.contains(&id) {
      changes
        .downcast_ref::<Arc<LinearBatchChanges<u32, T>>>()
        .unwrap()
        .clone()
    } else {
      consumer_ids.insert(id);
      // for any new watch created we emit full table

      let update_or_insert = self.internal.db.access_ecg_dyn(e_id, move |e| {
        e.access_component(c_id, move |c| {
          ComponentAccess {
            ecg: e.clone(),
            original: c.clone(),
            phantom: PhantomData::<T>,
          }
          .access()
          .iter_key_value()
          .map(|(k, v): (RawEntityHandle, T)| (k.index(), v.clone()))
          .collect::<Vec<_>>()
        })
        .unwrap()
      });

      Arc::new(LinearBatchChanges {
        removed: Default::default(),
        update_or_insert,
      })
    }
  }
}
