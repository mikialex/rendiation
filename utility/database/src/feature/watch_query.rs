use crate::*;

pub struct DBQueryChangeWatchGroup {
  internal: DBChangeWatchGroup,
}

// pub type DBView<V> = IterableComponentReadViewChecked<V>;
pub type DBChange<V> = Arc<FastHashMap<RawEntityHandle, ValueChange<V>>>;
// pub type DBComputeView<V> = (DBView<V>, DBChange<V>);

impl DBQueryChangeWatchGroup {
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

  pub fn get_buffered_changes<C: ComponentSemantic>(&mut self, id: u32) -> DBChange<C::Data> {
    self.get_buffered_changes_internal(id, C::Entity::entity_id(), C::component_id())
  }

  #[inline(never)] // remove the variant of component semantic to reduce the binary bloat
  fn get_buffered_changes_internal<T: CValue>(
    &mut self,
    id: u32,
    e_id: EntityId,
    c_id: ComponentId,
  ) -> DBChange<T> {
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

        let changes = if let Poll::Ready(Some(changes)) = rev.poll_impl(cx) {
          changes
        } else {
          Default::default()
        };
        Box::new(Arc::new(changes))
      });

    if consumer_ids.contains(&id) {
      let changes = changes.downcast_ref::<DBChange<T>>().unwrap().clone();

      changes
    } else {
      consumer_ids.insert(id);
      // for any new watch created we emit full table

      let full_view = self
        .internal
        .db
        .access_ecg_dyn(e_id, |ecg| {
          ecg.access_component(c_id, |c| IterableComponentReadViewChecked {
            ecg: ecg.clone(),
            read_view: c.read_untyped(),
            phantom: PhantomData,
          })
        })
        .unwrap();

      let full_view_as_delta = full_view
        .iter_key_value()
        .map(|(k, v)| (k, ValueChange::Delta(v, None)))
        .collect::<FastHashMap<_, _>>(); // todo avoid collect

      Arc::new(full_view_as_delta)
      // (full_view, Arc::new(full_view_as_delta))
    }
  }
}
