// use std::mem::ManuallyDrop;
// use std::pin::Pin;
// use std::task::{Context, Poll};

// use ::storage::IndexKeptVec;
// use futures::task::{noop_waker, AtomicWaker};
// use futures::Stream;

// use crate::*;

// #[derive(Clone)]
// pub struct DatabaseEntityRefCounting {
//   entity_ref_counts: Arc<RwLock<StreamMap<EntityId, EntityRefCount>>>,
//   /// map from entity id to foreign entity id, means this foreign entity owns multiple main
// entity.   reverse_ownership: Arc<RwLock<FastHashMap<EntityId, EntityId>>>,
//   cleanup_waker: Arc<AtomicWaker>,
// }

// impl DatabaseEntityRefCounting {
//   pub fn new(
//     db: Database,
//     mutation_watcher: DatabaseMutationWatch,
//     rev_ref: DatabaseEntityReverseReference,
//   ) -> Self {
//     // todo, we should do loop check in dep graph to warn if there is a circular dep

//     let entity_ref_counts: Arc<RwLock<StreamMap<EntityId, EntityRefCount>>> = Default::default();
//     let entity_ref_counts_ = entity_ref_counts.clone();
//     let reverse_ownership: Arc<RwLock<FastHashMap<EntityId, EntityId>>> = Default::default();
//     let r_o = reverse_ownership.clone();

//     let db_c = db.clone();

//     db_c.entity_meta_watcher.on(move |ecg| {
//       let erc = entity_ref_counts.clone();
//       let mut entity_ref_counts = entity_ref_counts.write();

//       let e_id = ecg.inner.type_id;

//       let watched_foreign_keys: Arc<RwLock<FastHashSet<ComponentId>>> = Default::default();
//       let w_f_k = watched_foreign_keys.clone();

//       let ref_data = EntityRefCount {
//         outer_refs: Default::default(),
//         inner_refs: Default::default(),
//         watched_foreign_keys: w_f_k,
//         db: db.clone(),
//         id: e_id,
//       };

//       entity_ref_counts.insert(e_id, ref_data);
//       drop(entity_ref_counts);

//       let mutation_watcher = mutation_watcher.clone();
//       let reverse_ownership = reverse_ownership.clone();
//       let rev_ref = rev_ref.clone();

//       ecg
//         .inner
//         .foreign_key_meta_watchers
//         .on(move |(s_id, f_e_id)| {
//           let entity_ref_counts = erc.read();
//           watched_foreign_keys.write().insert(*s_id);

//           if reverse_ownership.read().contains(&e_id) {
//             let ref_data = entity_ref_counts.get(&e_id).unwrap();
//             let changes = mutation_watcher
//               .watch_entity_set_dyn(*f_e_id)
//               .one_to_many_fanout(rev_ref.watch_inv_ref_dyn(*s_id, *f_e_id))
//               .key_as_value();
//             let changes = Box::new(changes);
//             ref_data.inner_refs.add_source(changes);
//           } else {
//             let ref_data = entity_ref_counts.get(f_e_id).unwrap();
//             let changes = mutation_watcher
//               .watch_dyn_foreign_key(*s_id, *f_e_id)
//               .collective_filter_map(|v| v.map(|v| v.index()));
//             let changes = Box::new(changes);
//             ref_data.inner_refs.add_source(changes);
//           };

//           false
//         });

//       false
//     });

//     Self {
//       entity_ref_counts: entity_ref_counts_,
//       cleanup_waker: Default::default(),
//       reverse_ownership: r_o,
//     }
//   }

//   // this function should be called before the database side foreign key declare.
//   pub fn declare_foreign_key_reverse_ownership<C: ForeignKeySemantic>(self) -> Self {
//     let refs = self.entity_ref_counts.read();
//     if let Some(ref_data) = refs.get(&C::ForeignEntity::entity_id()) {
//       let watched_foreign_keys = ref_data.watched_foreign_keys.read();
//       assert!(!watched_foreign_keys.contains(&C::component_id()));

//       // we add this info under the above guards
//       self
//         .reverse_ownership
//         .write()
//         .insert(C::Entity::entity_id(), C::ForeignEntity::entity_id());
//     }
//     assert!(refs.get(&C::ForeignEntity::entity_id()).is_none());
//     drop(refs);

//     self
//   }

//   pub fn cleanup_none_referenced_entities(&self) {
//     let waker = self.cleanup_waker.take().unwrap_or(noop_waker());
//     self.cleanup_waker.register(&waker);
//     let mut cx = Context::from_waker(&waker);
//     let mut entity_ref_counts = self.entity_ref_counts.write();
//     entity_ref_counts.poll_until_pending_not_care_result(&mut cx);
//   }

//   pub fn data_ref_ptr_creator<E: EntitySemantic>(&self) -> DataRefPtrWriteView<E> {
//     let entity_ref_counts = self.entity_ref_counts.read();
//     let entity_ref_count = entity_ref_counts.get(&E::entity_id()).unwrap();
//     let entity_checker = entity_ref_count
//       .outer_refs
//       .read()
//       .entity_checker
//       .make_read_holder();
//     DataRefPtrWriteView {
//       phantom_data: PhantomData,
//       ref_data: entity_ref_count.outer_refs.make_write_holder(),
//       entity_checker,
//     }
//   }
// }

// pub struct EntityRefCount {
//   outer_refs: Arc<RwLock<ExternalDataRefs>>,
//   // input multiple(any addressable idx => target entity idx)  -> target entity idx -> refcount
//   inner_refs: CollectionSetsRefcount<u32, u32>,
//   watched_foreign_keys: Arc<RwLock<FastHashSet<ComponentId>>>,
//   db: Database,
//   id: EntityId,
// }

// impl Stream for EntityRefCount {
//   type Item = ();

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//     let mut outer_count = self.outer_refs.write();
//     let outer_count: &mut ExternalDataRefs = &mut outer_count;

//     if let Poll::Ready(change) = self.inner_refs.poll_changes(cx) {
//       let mut entity_writer = self.db.entity_writer_untyped_dyn(self.id);

//       for (k, change) in change.iter_key_value() {
//         if change.is_removed() && outer_count.ref_counts.try_get(k).is_none() {
//           entity_writer.uncheck_handle_delete_entity(k);
//         }
//       }
//     }

//     let inner_count = self.inner_refs.access();
//     if !outer_count.clean_up_check_queue.is_empty() {
//       let mut entity_writer = self.db.entity_writer_untyped_dyn(self.id);
//       for idx in outer_count.clean_up_check_queue.drain(..) {
//         if !inner_count.contains(&idx) && outer_count.ref_counts.try_get(idx).is_none() {
//           entity_writer.uncheck_handle_delete_entity(idx);
//         }
//       }
//     }

//     Poll::Pending
//   }
// }

// #[derive(Default)]
// struct ExternalDataRefs {
//   entity_checker: Arc<RwLock<Arena<()>>>,
//   ref_counts: IndexKeptVec<u32>,
//   clean_up_check_queue: Vec<u32>,
//   waker: AtomicWaker,
// }

// pub struct DataRefPtrWriteView<T> {
//   phantom_data: PhantomData<T>,
//   ref_data: LockWriteGuardHolder<ExternalDataRefs>,
//   entity_checker: LockReadGuardHolder<Arena<()>>,
// }

// impl<T> DataRefPtrWriteView<T> {
//   pub fn create_ptr(&mut self, handle: EntityHandle<T>) -> DataRefPtr<T> {
//     if !self.entity_checker.contains(&(handle.handle.index())) {
//       panic!("handle is invalid")
//     }

//     self.ref_data.ref_counts.insert(1, handle.handle.index());

//     DataRefPtr {
//       inner: handle,
//       ref_count_storage: self.ref_data.get_lock(),
//     }
//   }
//   /// batch version of the drop logic of data ref ptr
//   pub fn drop_ptr(&mut self, handle: DataRefPtr<T>) {
//     let idx = handle.handle.index();
//     *self.ref_data.ref_counts.get_mut(idx) -= 1;
//     if *self.ref_data.ref_counts.get_mut(idx) == 0 {
//       self.ref_data.ref_counts.remove(idx);
//     }
//     self.ref_data.waker.wake();
//     self.ref_data.clean_up_check_queue.push(idx);
//     let _ = ManuallyDrop::new(handle);
//   }
//   /// batch version of the clone logic of data ref ptr
//   pub fn clone_ptr(&mut self, handle: &DataRefPtr<T>) -> DataRefPtr<T> {
//     *self.ref_data.ref_counts.get_mut(handle.handle.index()) += 1;
//     DataRefPtr {
//       inner: handle.inner,
//       ref_count_storage: handle.ref_count_storage.clone(),
//     }
//   }
// }

// impl<T> Drop for DataRefPtrWriteView<T> {
//   fn drop(&mut self) {
//     self.ref_data.waker.wake();
//   }
// }

// /// User could keep these ptr outside of the database to keep the entity alive;
// /// The foreign relations between different entities will be automatically considered.
// pub struct DataRefPtr<T> {
//   inner: EntityHandle<T>,
//   ref_count_storage: Arc<RwLock<ExternalDataRefs>>,
// }

// impl<T> Deref for DataRefPtr<T> {
//   type Target = EntityHandle<T>;

//   fn deref(&self) -> &Self::Target {
//     &self.inner
//   }
// }

// impl<T> Drop for DataRefPtr<T> {
//   fn drop(&mut self) {
//     let mut refs = self.ref_count_storage.write();
//     let idx = self.inner.handle.index();
//     let ref_count = refs.ref_counts.get_mut(idx);
//     assert!(*ref_count >= 1);
//     *ref_count -= 1;
//     if *ref_count == 0 {
//       refs.ref_counts.remove(idx);
//     }
//     refs.clean_up_check_queue.push(idx);
//     refs.waker.wake()
//   }
// }

// impl<T> Clone for DataRefPtr<T> {
//   fn clone(&self) -> Self {
//     let mut refs = self.ref_count_storage.write();
//     let ref_count = refs.ref_counts.get_mut(self.inner.handle.index());
//     *ref_count += 1;

//     Self {
//       inner: self.inner,
//       ref_count_storage: self.ref_count_storage.clone(),
//     }
//   }
// }
