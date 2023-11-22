use std::{hash::Hash, marker::PhantomData, sync::Arc};

use fast_hash_collection::*;
use parking_lot::RwLock;
use storage::{LinkListPool, ListHandle};

use crate::*;

pub trait ReactiveOneToManyRelationship<O, M>:
  VirtualMultiCollection<O, M> + ReactiveCollection<M, O>
{
}

impl<T, O, M> ReactiveOneToManyRelationship<O, M> for T where
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

pub trait ReactiveCollectionRelationExt<K, V>: Sized + 'static + ReactiveCollection<K, V> {
  fn into_one_to_many_by_hash(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    Self::Changes: Clone,
    K: Hash + Eq + Clone + 'static,
    V: Hash + Eq + Clone + 'static,
  {
    OneToManyRefHashBookKeeping {
      current_generation: 0,
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_hash_expose_type(self) -> OneToManyRefHashBookKeeping<V, K, Self>
  where
    Self::Changes: Clone,
    K: Hash + Eq + Clone + 'static,
    V: Hash + Eq + Clone + 'static,
  {
    OneToManyRefHashBookKeeping {
      current_generation: 0,
      upstream: self,
      mapping: Default::default(),
    }
  }

  fn into_one_to_many_by_idx(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    Self::Changes: Clone,
    K: LinearIdentification + Clone + 'static,
    V: LinearIdentification + Clone + 'static,
  {
    OneToManyRefDenseBookKeeping {
      current_generation: 0,
      upstream: self,
      mapping: Default::default(),
      phantom: PhantomData,
    }
  }

  fn into_one_to_many_by_idx_expose_type(self) -> OneToManyRefDenseBookKeeping<V, K, Self>
  where
    Self::Changes: Clone,
    K: LinearIdentification + Clone + 'static,
    V: LinearIdentification + Clone + 'static,
  {
    OneToManyRefDenseBookKeeping {
      current_generation: 0,
      upstream: self,
      mapping: Default::default(),
      phantom: PhantomData,
    }
  }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<MK, Relation>(self, relations: Relation) -> impl ReactiveCollection<MK, V>
  where
    V: Clone + 'static,
    MK: Clone + Eq + Hash + 'static,
    K: Clone + 'static,
    Relation: ReactiveOneToManyRelationship<K, MK> + 'static,
  {
    OneToManyFanout {
      upstream: self,
      relations,
      phantom: PhantomData,
    }
  }
}
impl<T, K, V> ReactiveCollectionRelationExt<K, V> for T where
  T: Sized + 'static + ReactiveCollection<K, V>
{
}

pub trait ReactiveCollectionRelationReduceExt<K>:
  Sized + 'static + ReactiveCollection<K, ()>
{
  fn many_to_one_reduce_key<SK, Relation>(
    self,
    relations: Relation,
  ) -> impl ReactiveCollection<SK, ()>
  where
    SK: Clone + Eq + Hash + 'static,
    K: Clone + 'static,
    Relation: ReactiveCollection<K, SK> + 'static,
  {
    ManyToOneReduce {
      upstream: self,
      relations,
      ref_counting: Default::default(),
      phantom: PhantomData,
    }
  }
}
impl<T, K> ReactiveCollectionRelationReduceExt<K> for T where
  T: Sized + 'static + ReactiveCollection<K, ()>
{
}

pub struct OneToManyFanout<O, M, X, Upstream, Relation>
where
  Upstream: ReactiveCollection<O, X>,
  Relation: ReactiveOneToManyRelationship<O, M>,
{
  upstream: Upstream,
  relations: Relation,
  phantom: PhantomData<(O, M, X)>,
}

impl<O, M, X, Upstream, Relation> ReactiveCollection<M, X>
  for OneToManyFanout<O, M, X, Upstream, Relation>
where
  M: Clone + Eq + Hash + 'static,
  X: Clone + 'static,
  O: Clone + 'static,
  Upstream: ReactiveCollection<O, X>,
  Relation: ReactiveOneToManyRelationship<O, M> + 'static,
{
  type Changes = impl Iterator<Item = CollectionDelta<M, X>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let relational_changes = self.relations.poll_changes(cx);
    let upstream_changes = self.upstream.poll_changes(cx);

    let mut output = FastHashMap::default(); // it's hard to predict capacity, should we compute it?
    if let Poll::Ready(Some(relational_changes)) = relational_changes {
      let getter = self.upstream.access();
      for change in relational_changes {
        match change {
          CollectionDelta::Delta(k, v, p) => {
            let p = p.and_then(|p| getter(&p));
            if let Some(v) = getter(&v) {
              output.insert(k.clone(), CollectionDelta::Delta(k, v, p));
            } else if let Some(p) = p {
              output.insert(k.clone(), CollectionDelta::Remove(k, p));
            }
          }
          CollectionDelta::Remove(k, p) => {
            if let Some(p) = getter(&p) {
              output.insert(k.clone(), CollectionDelta::Remove(k, p));
            }
          }
        }
      }
    }
    let inv_querier = self.relations.access_multi();
    if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
      for delta in upstream_changes {
        match delta {
          CollectionDelta::Remove(one, p) => inv_querier(&one, &mut |many| {
            output.insert(many.clone(), CollectionDelta::Remove(many, p.clone()));
          }),
          CollectionDelta::Delta(one, change, p) => inv_querier(&one, &mut |many| {
            output.insert(
              many.clone(),
              CollectionDelta::Delta(many, change.clone(), p.clone()),
            );
          }),
        }
      }
    }

    if output.is_empty() {
      Poll::Pending
    } else {
      let collected = output.into_values().collect::<Vec<_>>();
      Poll::Ready(Some(collected.into_iter()))
      // Poll::Ready(Some(output.into_values())) // todo, avoid collect
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
  M: Clone,
  X: Clone,
  O: Clone,
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

  fn iter_key(&self) -> impl Iterator<Item = M> + '_ {
    self.relations.iter_key()
  }
}

pub struct ManyToOneReduce<O, M, Upstream, Relation>
where
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveCollection<M, O>,
{
  upstream: Upstream,
  relations: Relation,
  ref_counting: FastHashMap<O, u32>,
  phantom: PhantomData<(O, M)>,
}

impl<O, M, Upstream, Relation> ReactiveCollection<O, ()>
  for ManyToOneReduce<O, M, Upstream, Relation>
where
  M: Clone + 'static,
  O: Clone + Eq + Hash + 'static,
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveCollection<M, O>,
{
  type Changes = impl Iterator<Item = CollectionDelta<O, ()>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let relational_changes = self.relations.poll_changes(cx);
    let upstream_changes = self.upstream.poll_changes(cx);

    let mut output = FastHashMap::default(); // it's hard to predict capacity, should we compute it?

    let getter = self.upstream.access();
    let one_acc = self.relations.access();

    if let Poll::Ready(Some(relational_changes)) = relational_changes {
      for change in relational_changes {
        let key = change.key();
        let old_value = change.old_value();
        let new_value = change.new_value();

        if let Some(ov) = old_value {
          if getter(key).is_some() {
            let rc = self.ref_counting.get_mut(ov).unwrap();
            *rc -= 1;
            if *rc == 0 {
              self.ref_counting.remove(ov);
              output.insert(ov.clone(), CollectionDelta::Remove(ov.clone(), ()));
            }
          }
        }

        if let Some(nv) = new_value {
          if getter(key).is_some() {
            let count = self.ref_counting.entry(nv.clone()).or_insert_with(|| {
              if let Some(CollectionDelta::Remove(..)) = output.get(nv) {
                // if contains remove, then cancel it
                output.remove(nv);
              } else {
                output.insert(nv.clone(), CollectionDelta::Delta(nv.clone(), (), None));
              }
              0
            });
            *count += 1;
          }
        }
      }
    }

    if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
      for delta in upstream_changes {
        match delta {
          CollectionDelta::Remove(many, _) => {
            if let Some(one) = one_acc(&many) {
              let rc = self.ref_counting.get_mut(&one).unwrap();
              *rc -= 1;
              if *rc == 0 {
                self.ref_counting.remove(&one);
                output.insert(one.clone(), CollectionDelta::Remove(one.clone(), ()));
              }
            }
          }
          CollectionDelta::Delta(many, _, _) => {
            if let Some(one) = one_acc(&many) {
              let count = self.ref_counting.entry(one.clone()).or_insert_with(|| {
                if let Some(CollectionDelta::Remove(..)) = output.get(&one) {
                  // if contains remove, then cancel it
                  output.remove(&one);
                } else {
                  output.insert(one.clone(), CollectionDelta::Delta(one.clone(), (), None));
                }
                0
              });
              *count += 1;
            }
          }
        }
      }
    }

    if output.is_empty() {
      Poll::Pending
    } else {
      let collected = output.into_values().collect::<Vec<_>>();
      Poll::Ready(Some(collected.into_iter()))
      // Poll::Ready(Some(output.into_values())) // todo, avoid collect
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    self.relations.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.ref_counting.shrink_to_fit(),
    }
  }
}

impl<O, M, Upstream, Relation> VirtualCollection<O, ()>
  for ManyToOneReduce<O, M, Upstream, Relation>
where
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveCollection<M, O>,
  O: Clone + Eq + Hash,
{
  fn iter_key(&self) -> impl Iterator<Item = O> + '_ {
    self.ref_counting.keys().cloned()
  }

  fn access(&self) -> impl Fn(&O) -> Option<()> + '_ {
    move |k| self.ref_counting.get(k).map(|_| {})
  }
}

pub struct OneToManyRefHashBookKeeping<O, M, T> {
  upstream: T,
  current_generation: u64,
  mapping: Arc<RwLock<(FastHashMap<O, FastHashSet<M>>, u64)>>,
}

impl<O, M, T: Clone> Clone for OneToManyRefHashBookKeeping<O, M, T> {
  fn clone(&self) -> Self {
    Self {
      current_generation: self.current_generation.clone(),
      upstream: self.upstream.clone(),
      mapping: self.mapping.clone(),
    }
  }
}

impl<O, M, T> VirtualCollection<M, O> for OneToManyRefHashBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
{
  fn iter_key(&self) -> impl Iterator<Item = M> + '_ {
    self.upstream.iter_key()
  }

  fn access(&self) -> impl Fn(&M) -> Option<O> + '_ {
    self.upstream.access()
  }
}

impl<O, M, T> VirtualMultiCollection<O, M> for OneToManyRefHashBookKeeping<O, M, T>
where
  M: Hash + Eq + Clone + 'static,
  O: Hash + Eq + Clone + 'static,
{
  fn iter_key_in_multi_collection(&self) -> impl Iterator<Item = O> + '_ {
    // todo, avoid clone
    self
      .mapping
      .read_recursive()
      .0
      .keys()
      .cloned()
      .collect::<Vec<_>>()
      .into_iter()
  }

  fn access_multi(&self) -> impl Fn(&O, &mut dyn FnMut(M)) + '_ {
    let mapping = self.mapping.read_recursive();
    move |o, visitor| {
      if let Some(set) = mapping.0.get(o) {
        for many in set.iter() {
          visitor(many.clone())
        }
      }
    }
  }
}

impl<O, M, T> ReactiveCollection<M, O> for OneToManyRefHashBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
  T::Changes: Clone,
  M: Hash + Eq + Clone + 'static,
  O: Hash + Eq + Clone + 'static,
{
  type Changes = T::Changes;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self.upstream.poll_changes(cx);
    self.current_generation += 1;

    if let Poll::Ready(Some(changes)) = r.clone() {
      for change in changes {
        let mut mapping = self.mapping.write();
        if mapping.1 < self.current_generation {
          mapping.1 = self.current_generation;

          let many = change.key().clone();
          let new_one = change.new_value();

          let old_refed_one = change.old_value();
          // remove possible old relations
          if let Some(old_refed_one) = old_refed_one {
            let previous_one_refed_many = mapping.0.get_mut(old_refed_one).unwrap();
            previous_one_refed_many.remove(&many);
            if previous_one_refed_many.is_empty() {
              mapping.0.remove(old_refed_one);
            }
          }

          // setup new relations
          if let Some(new_one) = new_one {
            let new_one_refed_many = mapping.0.entry(new_one.clone()).or_default();
            new_one_refed_many.insert(many.clone());
          }
        }
      }
    }

    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => self.mapping.write().0.shrink_to_fit(),
    }
  }
}

pub struct OneToManyRefDenseBookKeeping<O, M, T> {
  upstream: T,
  current_generation: u64,
  mapping: Arc<RwLock<Mapping>>,
  phantom: PhantomData<(O, M)>,
}

impl<O, M, T: Clone> Clone for OneToManyRefDenseBookKeeping<O, M, T> {
  fn clone(&self) -> Self {
    Self {
      current_generation: self.current_generation.clone(),
      upstream: self.upstream.clone(),
      mapping: self.mapping.clone(),
      phantom: PhantomData,
    }
  }
}

#[derive(Default)]
struct Mapping {
  generation: u64,
  mapping_buffer: LinkListPool<u32>,
  mapping: Vec<ListHandle>,
}

impl<O, M, T> VirtualCollection<M, O> for OneToManyRefDenseBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
{
  fn iter_key(&self) -> impl Iterator<Item = M> + '_ {
    self.upstream.iter_key()
  }

  fn access(&self) -> impl Fn(&M) -> Option<O> + '_ {
    self.upstream.access()
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
      .read_recursive()
      .mapping
      .iter()
      .enumerate()
      .filter_map(|(i, list)| list.is_empty().then_some(O::from_alloc_index(i as u32)))
      .collect::<Vec<_>>()
      .into_iter()
  }

  fn access_multi(&self) -> impl Fn(&O, &mut dyn FnMut(M)) + '_ {
    let mapping = self.mapping.read_recursive();
    move |o, visitor| {
      if let Some(list) = mapping.mapping.get(o.alloc_index() as usize) {
        mapping.mapping_buffer.visit(list, |v, _| {
          visitor(M::from_alloc_index(*v));
          true
        })
      }
    }
  }
}

impl<O, M, T> ReactiveCollection<M, O> for OneToManyRefDenseBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
  T::Changes: Clone,
  M: LinearIdentification + Clone + 'static,
  O: LinearIdentification + Clone + 'static,
{
  type Changes = T::Changes;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let r = self.upstream.poll_changes(cx);
    self.current_generation += 1;

    if let Poll::Ready(Some(changes)) = r.clone() {
      for change in changes {
        let mut mapping = self.mapping.write();
        if mapping.generation < self.current_generation {
          mapping.generation = self.current_generation;
          let mapping: &mut Mapping = &mut mapping;
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
            mapping.mapping[new_one.alloc_index() as usize] = ListHandle::default();
            mapping.mapping_buffer.insert(
              &mut mapping.mapping[new_one.alloc_index() as usize],
              new_one.alloc_index(),
            );
          }
        }
      }
    }

    r
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.upstream.extra_request(request);
    match request {
      ExtraCollectionOperation::MemoryShrinkToFit => {
        let mut mapping = self.mapping.write();
        mapping.mapping.shrink_to_fit();
        mapping.mapping_buffer.shrink_to_fit();
      }
    }
  }
}
