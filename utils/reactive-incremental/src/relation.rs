use std::{hash::Hash, marker::PhantomData, ops::DerefMut};

use fast_hash_collection::*;
use storage::{LinkListPool, ListHandle};

use crate::*;

pub trait ReactiveOneToManyRelationship<O: Send, M: Send>:
  VirtualMultiCollection<O, M> + ReactiveCollection<M, O>
{
}

impl<T, O: Send, M: Send> ReactiveOneToManyRelationship<O, M> for T where
  T: VirtualMultiCollection<O, M> + ReactiveCollection<M, O>
{
}

pub trait DynamicReactiveOneToManyRelationship<O, M>:
  DynamicVirtualMultiCollection<O, M> + DynamicReactiveCollection<M, O>
{
}
impl<T, O, M> DynamicReactiveOneToManyRelationship<O, M> for T where
  T: DynamicVirtualMultiCollection<O, M> + DynamicReactiveCollection<M, O>
{
}
impl<O, M> VirtualCollection<M, O> for Box<dyn DynamicReactiveOneToManyRelationship<O, M>> {
  fn iter_key(&self) -> impl Iterator<Item = M> + '_ {
    self.deref().iter_key_boxed()
  }

  fn access(&self) -> impl Fn(&M) -> Option<O> + Sync + '_ {
    self.deref().access_boxed()
  }

  fn try_access(&self) -> Option<Box<dyn Fn(&M) -> Option<O> + Sync + '_>> {
    self.deref().try_access_boxed()
  }
}
impl<O, M> VirtualMultiCollection<O, M> for Box<dyn DynamicReactiveOneToManyRelationship<O, M>> {
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = O> + '_ {
    self.deref().iter_key_in_multi_collection_boxed()
  }

  fn access_multi(&self) -> impl Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_ {
    self.deref().access_multi_boxed()
  }

  fn try_access_multi(&self) -> Option<Box<dyn Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_>> {
    self.deref().try_access_multi_boxed()
  }
}
impl<O, M> ReactiveCollection<M, O> for Box<dyn DynamicReactiveOneToManyRelationship<O, M>>
where
  O: Clone + Send + Sync + 'static,
  M: Clone + Send + Sync + 'static,
{
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<M, O>> {
    self.deref_mut().poll_changes_dyn(cx)
  }
  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.deref_mut().extra_request_dyn(request)
  }
}

pub trait ReactiveCollectionRelationExt<K: Send, V: Send>:
  Sized + 'static + ReactiveCollection<K, V>
{
  fn into_one_to_many_by_hash(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    CollectionChanges<K, V>: Clone,
    K: Hash + Eq + Clone + Sync + 'static,
    V: Hash + Eq + Clone + Sync + 'static,
  {
    OneToManyRefHashBookKeeping {
      upstream: BufferedCollection::new(self),
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_hash_expose_type(self) -> OneToManyRefHashBookKeeping<V, K, Self>
  where
    CollectionChanges<K, V>: Clone,
    K: Hash + Eq + Clone + 'static,
    V: Hash + Eq + Clone + 'static,
  {
    OneToManyRefHashBookKeeping {
      upstream: BufferedCollection::new(self),
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_idx(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    CollectionChanges<K, V>: Clone,
    K: LinearIdentification + Eq + std::hash::Hash + Clone + Sync + 'static,
    V: LinearIdentification + Clone + Sync + 'static,
  {
    OneToManyRefDenseBookKeeping {
      upstream: BufferedCollection::new(self),
      mapping: Default::default(),
      phantom: PhantomData,
    }
  }

  fn into_one_to_many_by_idx_expose_type(self) -> OneToManyRefDenseBookKeeping<V, K, Self>
  where
    CollectionChanges<K, V>: Clone,
    K: LinearIdentification + Clone + 'static,
    V: LinearIdentification + Clone + 'static,
  {
    OneToManyRefDenseBookKeeping {
      upstream: self,
      mapping: Default::default(),
      phantom: PhantomData,
    }
  }
}
impl<T, K, V> ReactiveCollectionRelationExt<K, V> for T
where
  T: Sized + 'static + ReactiveCollection<K, V>,
  K: Send,
  V: Send,
{
}

pub trait ReactiveCollectionRelationReduceExt<K: Send>:
  Sized + 'static + ReactiveCollection<K, ()>
{
  fn many_to_one_reduce_key<SK, Relation>(
    self,
    relations: Relation,
  ) -> impl ReactiveCollection<SK, ()>
  where
    SK: Clone + Eq + Hash + Send + Sync + 'static,
    K: Clone + Eq + Hash + Sync + 'static,
    Relation: ReactiveCollection<K, SK> + 'static,
  {
    ManyToOneReduce {
      upstream: BufferedCollection::new(self),
      relations: BufferedCollection::new(relations),
      phantom: PhantomData,
      ref_count: Default::default(),
    }
  }
}
impl<T, K: Send> ReactiveCollectionRelationReduceExt<K> for T where
  T: Sized + 'static + ReactiveCollection<K, ()>
{
}

pub struct OneToManyFanout<O, M, X, Upstream, Relation>
where
  Upstream: ReactiveCollection<O, X>,
  Relation: ReactiveOneToManyRelationship<O, M>,
  M: Send,
  O: Send,
  X: Send,
{
  pub(crate) upstream: BufferedCollection<CollectionChanges<O, X>, Upstream>,
  pub(crate) relations: BufferedCollection<CollectionChanges<M, O>, Relation>,
  pub(crate) phantom: PhantomData<(O, M, X)>,
}

impl<O, M, X, Upstream, Relation> ReactiveCollection<M, X>
  for OneToManyFanout<O, M, X, Upstream, Relation>
where
  M: Clone + Eq + Hash + Send + Sync + 'static,
  X: Clone + Send + Sync + 'static,
  O: Clone + Eq + Hash + Send + Sync + 'static,
  Upstream: ReactiveCollection<O, X>,
  Relation: ReactiveOneToManyRelationship<O, M> + 'static,
{
  #[tracing::instrument(skip_all, name = "OneToManyFanout")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<M, X>> {
    let waker = cx.waker().clone();
    let (relational_changes, upstream_changes) = rayon::join(
      || {
        let mut cx = Context::from_waker(&waker);
        self.relations.poll_changes(&mut cx)
      },
      || {
        let mut cx = Context::from_waker(&waker);
        self.upstream.poll_changes(&mut cx)
      },
    );

    if relational_changes.is_blocked() {
      if let CPoll::Ready(v) = upstream_changes {
        self.upstream.put_back_to_buffered(v);
      }
      return CPoll::Blocked;
    }
    if upstream_changes.is_blocked() {
      if let CPoll::Ready(v) = relational_changes {
        self.relations.put_back_to_buffered(v);
      }
      return CPoll::Blocked;
    }

    let getter = self.upstream.try_access();
    let inv_querier = self.relations.try_access_multi();

    if getter.is_none() || inv_querier.is_none() {
      drop(getter);
      drop(inv_querier);
      if let CPoll::Ready(v) = relational_changes {
        self.relations.put_back_to_buffered(v);
      }
      if let CPoll::Ready(v) = upstream_changes {
        self.upstream.put_back_to_buffered(v);
      }
      return CPoll::Blocked;
    };
    let getter = getter.unwrap();
    let inv_querier = inv_querier.unwrap();

    let getter_previous = make_previous(&upstream_changes, &getter);

    let mut output = FastHashMap::default();
    if let CPoll::Ready(relational_changes) = relational_changes {
      relational_changes
        .into_values()
        .for_each(|change| match change {
          CollectionDelta::Delta(k, v, p) => {
            // to get the real previous X, we need the previous o->x mapping
            let p = p.and_then(|p| getter_previous(&p));
            if let Some(v) = getter(&v) {
              output.insert(k.clone(), CollectionDelta::Delta(k, v, p));
            } else if let Some(p) = p {
              output.insert(k.clone(), CollectionDelta::Remove(k, p));
            }
          }
          CollectionDelta::Remove(k, p) => {
            if let Some(p) = getter_previous(&p) {
              output.insert(k.clone(), CollectionDelta::Remove(k, p));
            }
          }
        });
    }
    if let CPoll::Ready(upstream_changes) = &upstream_changes {
      for delta in upstream_changes.values() {
        // the inv_query is the current relation, the previous one's delta is emitted
        // by the above relation change code
        match delta {
          CollectionDelta::Remove(one, p) => inv_querier(one, &mut |many| {
            output.insert(many.clone(), CollectionDelta::Remove(many, p.clone()));
          }),
          CollectionDelta::Delta(one, change, p) => inv_querier(one, &mut |many| {
            output.insert(
              many.clone(),
              CollectionDelta::Delta(many, change.clone(), p.clone()),
            );
          }),
        }
      }
    }

    if output.is_empty() {
      CPoll::Pending
    } else {
      CPoll::Ready(output)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    self.relations.extra_request(request);
  }
}

impl<O, M, X, Upstream, Relation> VirtualCollection<M, X>
  for OneToManyFanout<O, M, X, Upstream, Relation>
where
  M: Clone + Send + Sync,
  X: Clone + Send + Sync,
  O: Clone + Send + Sync,
  Upstream: ReactiveCollection<O, X>,
  Relation: ReactiveOneToManyRelationship<O, M>,
{
  fn access(&self) -> impl Fn(&M) -> Option<X> + '_ {
    let upstream_getter = self.upstream.access();
    let access = self.relations.access();
    move |key| {
      let one = access(key)?;
      upstream_getter(&one)
    }
  }
  fn try_access(&self) -> Option<Box<dyn Fn(&M) -> Option<X> + Sync + '_>> {
    let upstream_getter = self.upstream.try_access()?;
    let access = self.relations.try_access()?;
    let acc = move |key: &_| {
      let one = access(key)?;
      upstream_getter(&one)
    };
    let boxed = Box::new(acc) as Box<dyn Fn(&M) -> Option<X> + Sync + '_>;
    boxed.into()
  }

  fn iter_key(&self) -> impl Iterator<Item = M> + '_ {
    self.relations.iter_key()
  }
}

pub struct ManyToOneReduce<O, M, Upstream, Relation>
where
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveCollection<M, O>,
  M: Send,
  O: Send,
{
  upstream: BufferedCollection<CollectionChanges<M, ()>, Upstream>,
  relations: BufferedCollection<CollectionChanges<M, O>, Relation>,
  phantom: PhantomData<(O, M)>,
  ref_count: FastHashMap<O, u32>,
}

impl<O, M, Upstream, Relation> ReactiveCollection<O, ()>
  for ManyToOneReduce<O, M, Upstream, Relation>
where
  M: Clone + Send + Eq + Hash + Sync + 'static,
  O: Clone + Eq + Hash + Send + Sync + 'static,
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveCollection<M, O>,
{
  #[tracing::instrument(skip_all, name = "ManyToOneReduce")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<O, ()>> {
    let waker = cx.waker().clone();
    let (relational_changes, upstream_changes) = rayon::join(
      || {
        let mut cx = Context::from_waker(&waker);
        self.relations.poll_changes(&mut cx)
      },
      || {
        let mut cx = Context::from_waker(&waker);
        self.upstream.poll_changes(&mut cx)
      },
    );

    if relational_changes.is_blocked() {
      if let CPoll::Ready(v) = upstream_changes {
        self.upstream.put_back_to_buffered(v);
      }
      return CPoll::Blocked;
    }
    if upstream_changes.is_blocked() {
      if let CPoll::Ready(v) = relational_changes {
        self.relations.put_back_to_buffered(v);
      }
      return CPoll::Blocked;
    }

    let getter = self.upstream.try_access();
    let one_acc = self.relations.try_access();

    if getter.is_none() || one_acc.is_none() {
      drop(getter);
      drop(one_acc);
      if let CPoll::Ready(v) = relational_changes {
        self.relations.put_back_to_buffered(v);
      }
      if let CPoll::Ready(v) = upstream_changes {
        self.upstream.put_back_to_buffered(v);
      }
      return CPoll::Blocked;
    };
    let getter = getter.unwrap();
    let one_acc = one_acc.unwrap();

    let getter_previous = make_previous(&upstream_changes, &getter);

    let mut output = FastHashMap::default();

    if let CPoll::Ready(relational_changes) = &relational_changes {
      for change in relational_changes.values() {
        let key = change.key();
        let old_value = change.old_value();
        let new_value = change.new_value();

        if let Some(ov) = old_value {
          if getter_previous(key).is_some() {
            let ref_count = self.ref_count.get_mut(ov).unwrap();
            *ref_count -= 1;
            if *ref_count == 0 {
              self.ref_count.remove(ov);
              output.insert(ov.clone(), CollectionDelta::Remove(ov.clone(), ()));
            }
          }
        }

        if let Some(nv) = new_value {
          if getter_previous(key).is_some() {
            let ref_count = self.ref_count.entry(nv.clone()).or_insert_with(|| {
              output.insert(nv.clone(), CollectionDelta::Delta(nv.clone(), (), None));
              0
            });
            *ref_count += 1;
          }
        }
      }
    }

    if let CPoll::Ready(upstream_changes) = &upstream_changes {
      for delta in upstream_changes.values() {
        match delta {
          CollectionDelta::Remove(many, _) => {
            // we should remove from the new old relation
            if let Some(one) = one_acc(many) {
              if let Some(ref_count) = self.ref_count.get_mut(&one) {
                *ref_count -= 1;
                if *ref_count == 0 {
                  self.ref_count.remove(&one);

                  if let Some(CollectionDelta::Delta(_, _, _)) = output.get(&one) {
                    // cancel out
                    output.remove(&one);
                  } else {
                    output.insert(one.clone(), CollectionDelta::Remove(one.clone(), ()));
                  }
                }
              }
            }
          }
          CollectionDelta::Delta(many, _, p) => {
            if p.is_none() {
              // should check if it is insert
              // we should insert into the new directed relation
              if let Some(one) = one_acc(many) {
                let ref_count = self.ref_count.entry(one.clone()).or_insert_with(|| {
                  if let Some(CollectionDelta::Remove(_, _)) = output.get(&one) {
                    // cancel out
                    output.remove(&one);
                  } else {
                    output.insert(one.clone(), CollectionDelta::Delta(one.clone(), (), None));
                  }
                  0
                });
                *ref_count += 1;
              }
            }
          }
        }
      }
    }

    if output.is_empty() {
      CPoll::Pending
    } else {
      CPoll::Ready(output)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    self.relations.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => {
        self.ref_count.shrink_to_fit();
      }
    }
  }
}

impl<O, M, Upstream, Relation> VirtualCollection<O, ()>
  for ManyToOneReduce<O, M, Upstream, Relation>
where
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveCollection<M, O>,
  O: Clone + Eq + Hash + Send + Sync,
  M: Send + Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = O> + '_ {
    self.ref_count.keys().cloned()
  }

  fn access(&self) -> impl Fn(&O) -> Option<()> + '_ {
    move |k| self.ref_count.get(k).map(|_| {})
  }

  fn try_access(&self) -> Option<Box<dyn Fn(&O) -> Option<()> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&O) -> Option<()> + Sync + '_>;
    boxed.into()
  }
}

pub struct OneToManyRefHashBookKeeping<O, M, T> {
  upstream: BufferedCollection<CollectionChanges<M, O>, T>,
  mapping: FastHashMap<O, FastHashSet<M>>,
}

impl<O: Clone, M: Clone, T: Clone> Clone for OneToManyRefHashBookKeeping<O, M, T> {
  fn clone(&self) -> Self {
    Self {
      upstream: self.upstream.clone(),
      mapping: self.mapping.clone(),
    }
  }
}

impl<O, M, T> VirtualCollection<M, O> for OneToManyRefHashBookKeeping<O, M, T>
where
  T: VirtualCollection<M, O>,
  M: Send + Sync,
  O: Send + Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = M> + '_ {
    self.upstream.iter_key()
  }

  fn access(&self) -> impl Fn(&M) -> Option<O> + '_ {
    self.upstream.access()
  }
  fn try_access(&self) -> Option<Box<dyn Fn(&M) -> Option<O> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&M) -> Option<O> + Sync + '_>;
    boxed.into()
  }
}

impl<O, M, T> VirtualMultiCollection<O, M> for OneToManyRefHashBookKeeping<O, M, T>
where
  M: Hash + Eq + Clone + Send + Sync + 'static,
  O: Hash + Eq + Clone + Send + Sync + 'static,
  T: Sync,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = O> + '_ {
    // todo, avoid clone
    self.mapping.keys().cloned().collect::<Vec<_>>().into_iter()
  }

  fn access_multi(&self) -> impl Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_ {
    move |o, visitor| {
      if let Some(set) = self.mapping.get(o) {
        for many in set.iter() {
          visitor(many.clone())
        }
      }
    }
  }
  fn try_access_multi(&self) -> Option<Box<dyn Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_>> {
    let acc = self.access_multi();
    let boxed = Box::new(acc) as Box<dyn Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_>;
    boxed.into()
  }
}

impl<O, M, T> ReactiveCollection<M, O> for OneToManyRefHashBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
  M: Hash + Eq + Clone + Send + Sync + 'static,
  O: Hash + Eq + Clone + Send + Sync + 'static,
{
  #[tracing::instrument(skip_all, name = "OneToManyRefHashBookKeeping")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<M, O>> {
    let r = self.upstream.poll_changes(cx);

    if let CPoll::Ready(changes) = r.clone() {
      let mapping = &mut self.mapping;

      for change in changes.into_values() {
        let many = change.key().clone();
        let new_one = change.new_value();

        let old_refed_one = change.old_value();
        // remove possible old relations
        if let Some(old_refed_one) = old_refed_one {
          let previous_one_refed_many = mapping.get_mut(old_refed_one).unwrap();
          previous_one_refed_many.remove(&many);
          if previous_one_refed_many.is_empty() {
            mapping.remove(old_refed_one);
          }
        }

        // setup new relations
        if let Some(new_one) = new_one {
          let new_one_refed_many = mapping.entry(new_one.clone()).or_default();
          new_one_refed_many.insert(many.clone());
        }
      }
    }

    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.mapping.shrink_to_fit(),
    }
  }
}

pub struct OneToManyRefDenseBookKeeping<O, M, T> {
  upstream: T,
  mapping: Mapping,
  phantom: PhantomData<(O, M)>,
}

#[derive(Default)]
struct Mapping {
  mapping_buffer: LinkListPool<u32>,
  mapping: Vec<ListHandle>,
}

impl<O, M, T> VirtualCollection<M, O> for OneToManyRefDenseBookKeeping<O, M, T>
where
  T: VirtualCollection<M, O>,
  M: Send + Sync,
  O: Send + Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = M> + '_ {
    self.upstream.iter_key()
  }

  fn access(&self) -> impl Fn(&M) -> Option<O> + '_ {
    self.upstream.access()
  }
  fn try_access(&self) -> Option<Box<dyn Fn(&M) -> Option<O> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&M) -> Option<O> + Sync + '_>;
    boxed.into()
  }
}

impl<O, M, T> VirtualMultiCollection<O, M> for OneToManyRefDenseBookKeeping<O, M, T>
where
  M: LinearIdentification + Clone + 'static,
  O: LinearIdentification + Clone + 'static,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = O> + '_ {
    // todo, avoid clone
    self
      .mapping
      .mapping
      .iter()
      .enumerate()
      .filter_map(|(i, list)| list.is_empty().then_some(O::from_alloc_index(i as u32)))
      .collect::<Vec<_>>()
      .into_iter()
  }

  fn access_multi(&self) -> impl Fn(&O, &mut dyn FnMut(M)) + '_ {
    let mapping = &self.mapping;
    move |o, visitor| {
      if let Some(list) = mapping.mapping.get(o.alloc_index() as usize) {
        mapping.mapping_buffer.visit(list, |v, _| {
          visitor(M::from_alloc_index(*v));
          true
        })
      }
    }
  }
  fn try_access_multi(&self) -> Option<Box<dyn Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_>> {
    let acc = self.access_multi();
    let boxed = Box::new(acc) as Box<dyn Fn(&O, &mut dyn FnMut(M)) + Send + Sync + '_>;
    boxed.into()
  }
}

impl<O, M, T> ReactiveCollection<M, O> for OneToManyRefDenseBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
  M: LinearIdentification + Eq + std::hash::Hash + Clone + Send + Sync + 'static,
  O: LinearIdentification + Clone + Send + Sync + 'static,
{
  #[tracing::instrument(skip_all, name = "OneToManyRefDenseBookKeeping")]
  fn poll_changes(&mut self, cx: &mut Context<'_>) -> CPoll<CollectionChanges<M, O>> {
    let r = self.upstream.poll_changes(cx);

    if let CPoll::Ready(changes) = r.clone() {
      for change in changes.into_values() {
        let mapping: &mut Mapping = &mut self.mapping;
        let many = *change.key();
        let new_one = change.new_value();

        let old_refed_one = change.old_value();
        // remove possible old relations
        if let Some(old_refed_one) = old_refed_one {
          let previous_one_refed_many = mapping
            .mapping
            .get_mut(old_refed_one.alloc_index() as usize)
            .unwrap();

          //  this is O(n), should we care about it?
          mapping
            .mapping_buffer
            .visit_and_remove(previous_one_refed_many, |value, _| {
              let should_remove = *value == many.alloc_index();
              (should_remove, !should_remove)
            });
        }

        // setup new relations
        if let Some(new_one) = &new_one {
          let alloc_index = new_one.alloc_index() as usize;
          if alloc_index >= mapping.mapping.len() {
            mapping
              .mapping
              .resize(alloc_index + 1, ListHandle::default());
          }

          mapping.mapping_buffer.insert(
            &mut mapping.mapping[new_one.alloc_index() as usize],
            many.alloc_index(),
          );
        }
      }
    }

    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => {
        let mapping = &mut self.mapping;
        mapping.mapping.shrink_to_fit();
        mapping.mapping_buffer.shrink_to_fit();
      }
    }
  }
}
