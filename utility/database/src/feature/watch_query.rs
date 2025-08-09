use crate::*;

pub struct DBQueryChangeWatchGroup {
  producers: FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>,
  consumers: FastHashMap<ComponentId, FastHashSet<u32>>,
  current_results: FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>,
  next_consumer_id: u32,
  pub(crate) db: Database,
}

type DBView<V> = IterableComponentReadViewChecked<V>;
type DBChange<V> = Arc<FastHashMap<RawEntityHandle, ValueChange<V>>>;
type DBComputeView<V> = (DBView<V>, DBChange<V>);

impl DBQueryChangeWatchGroup {
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

  pub fn get_buffered_changes<C: ComponentSemantic>(&mut self, id: u32) -> DBComputeView<C::Data> {
    self.get_buffered_changes_internal(id, C::Entity::entity_id(), C::component_id())
  }

  #[inline(never)] // remove the variant of component semantic to reduce the binary bloat
  fn get_buffered_changes_internal<T: CValue>(
    &mut self,
    id: u32,
    e_id: EntityId,
    c_id: ComponentId,
  ) -> DBComputeView<T> {
    let rev = self.producers.entry(c_id).or_insert_with(|| {
      let rev = self.db.access_ecg_dyn(e_id, move |e| {
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

    let consumer_ids = self.consumers.entry(c_id).or_default();

    let changes = self.current_results.entry(c_id).or_insert_with(|| {
      noop_ctx!(cx);

      let changes = if let Poll::Ready(Some(changes)) = rev.poll_impl(cx) {
        changes
      } else {
        Default::default()
      };
      Box::new(Arc::new(changes))
    });

    let full_view = self
      .db
      .access_ecg_dyn(e_id, |ecg| {
        ecg.access_component(c_id, |c| IterableComponentReadViewChecked {
          ecg: ecg.clone(),
          read_view: c.read_untyped(),
          phantom: PhantomData,
        })
      })
      .unwrap();

    if consumer_ids.contains(&id) {
      let changes = changes.downcast_ref::<DBChange<T>>().unwrap().clone();

      (full_view, changes)
    } else {
      consumer_ids.insert(id);
      // for any new watch created we emit full table

      let full_view_as_delta = full_view
        .iter_key_value()
        .map(|(k, v)| (k, ValueChange::Delta(v, None)))
        .collect::<FastHashMap<_, _>>();

      (full_view, Arc::new(full_view_as_delta))
    }
  }
}
