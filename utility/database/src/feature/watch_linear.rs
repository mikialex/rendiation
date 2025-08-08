use crate::*;

pub struct DBLinearChangeWatchGroup {
  producers: FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>,
  consumers: FastHashMap<ComponentId, FastHashSet<u32>>,
  current_results: FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>,
  next_consumer_id: u32,
  pub(crate) db: Database,
}

impl DBLinearChangeWatchGroup {
  pub fn new(db: &Database) -> Self {
    Self {
      producers: Default::default(),
      consumers: Default::default(),
      current_results: Default::default(),
      next_consumer_id: 0,
      db: db.clone(),
    }
  }

  pub fn clear_changes(&mut self) {
    self.current_results.clear();
  }

  pub fn allocate_next_consumer_id(&mut self) -> u32 {
    self.next_consumer_id += 1;
    self.next_consumer_id
  }

  pub fn notify_consumer_dropped(&mut self, component_id: ComponentId, consumer_id: u32) {
    let consumers = self.consumers.get_mut(&component_id).unwrap();
    let removed = consumers.remove(&consumer_id);
    assert!(removed);
    if consumers.is_empty() {
      self.producers.remove(&component_id); // stop the watch
      self.consumers.remove(&component_id);
    }
  }

  pub fn get_buffered_changes<C: ComponentSemantic>(
    &mut self,
    id: u32,
  ) -> Arc<LinearBatchChanges<C::Data>> {
    let rev = self.producers.entry(C::component_id()).or_insert_with(|| {
      let rev = self.db.access_ecg_dyn(C::Entity::entity_id(), move |e| {
        e.access_component(C::component_id(), move |c| {
          add_listen(
            ComponentAccess {
              ecg: e.clone(),
              original: c.clone(),
              phantom: PhantomData::<C::Data>,
            },
            &c.data_watchers,
          )
        })
        .unwrap()
      });
      Box::new(rev)
    });

    let rev = rev
      .downcast_mut::<CollectiveMutationReceiver<RawEntityHandle, <C as ComponentSemantic>::Data>>()
      .unwrap();

    let consumer_ids = self.consumers.entry(C::component_id()).or_default();

    let changes = self
      .current_results
      .entry(C::component_id())
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
        .downcast_ref::<Arc<LinearBatchChanges<C::Data>>>()
        .unwrap()
        .clone()
    } else {
      consumer_ids.insert(id);
      // for any new watch created we emit full table

      let update_or_insert = self.db.access_ecg_dyn(C::Entity::entity_id(), move |e| {
        e.access_component(C::component_id(), move |c| {
          ComponentAccess {
            ecg: e.clone(),
            original: c.clone(),
            phantom: PhantomData::<C::Data>,
          }
          .access()
          .iter_key_value()
          .map(|(k, v): (RawEntityHandle, C::Data)| (k.index(), v.clone()))
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
