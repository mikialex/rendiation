use std::{hash::Hash, marker::PhantomData};

use fast_hash_collection::*;
use storage::{LinkListPool, ListHandle};

use crate::*;

/// Implementation should guarantee for each v -> k, have the bijection of k -> v;
///
/// we could use ReactiveCollection<A, B>+ ReactiveCollection<B, A> as parent bound but rust think
/// it will impl conflict. and we also want avoid unnecessary stream fork
pub trait ReactiveOneToOneRelationship<A, B>: ReactiveCollection<A, B> {
  fn iter_by_value_one_one(&self, skip_cache: bool) -> impl Iterator<Item = B> + '_;
  fn access_by_value_one_one(&self, skip_cache: bool) -> impl Fn(&B) -> Option<A> + '_;
}

pub trait ReactiveOneToManyRelationship<O, M>:
  VirtualMultiCollection<O, M> + ReactiveCollection<M, O>
{
}

impl<T, O, M> ReactiveOneToManyRelationship<O, M> for T where
  T: VirtualMultiCollection<O, M> + ReactiveCollection<M, O>
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
      upstream: self,
      mapping: Default::default(),
      rev_mapping: Default::default(),
    }
  }
  fn into_one_to_many_by_idx(self) -> impl ReactiveOneToManyRelationship<V, K>
  where
    Self::Changes: Clone,
    K: LinearIdentification + Clone + 'static,
    V: LinearIdentification + Clone + 'static,
  {
    OneToManyRefDenseBookKeeping {
      upstream: self,
      mapping_buffer: Default::default(),
      mapping: Default::default(),
      rev_mapping: Default::default(),
      phantom: PhantomData,
    }
  }

  // fn cast_into_one_to_one(self) -> impl ReactiveOneToOneRelationship<V, K>
  // where
  //   K: Eq,
  //   V: Eq,
  // {
  //   todo!()
  // }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<MK, Relation>(self, relations: Relation) -> impl ReactiveCollection<MK, V>
  where
    V: Clone + 'static,
    MK: Clone + 'static,
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
  fn many_to_one_key_reduce<SK, Relation>(
    self,
    relations: Relation,
  ) -> impl ReactiveCollection<SK, ()>
  where
    SK: Clone + 'static,
    K: Clone + 'static,
    Relation: ReactiveOneToManyRelationship<SK, K> + 'static,
  {
    ManyToOneReduce {
      upstream: self,
      relations,
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
  M: Clone + 'static,
  X: Clone + 'static,
  O: Clone + 'static,
  Upstream: ReactiveCollection<O, X>,
  Relation: ReactiveOneToManyRelationship<O, M> + 'static,
{
  type Changes = impl Iterator<Item = CollectionDelta<M, X>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let relational_changes = self.relations.poll_changes(cx);
    let upstream_changes = self.upstream.poll_changes(cx);

    let mut output = Vec::new(); // it's hard to predict capacity, should we compute it?
    if let Poll::Ready(Some(relational_changes)) = relational_changes {
      let getter = self.upstream.access(false);
      for change in relational_changes {
        match change {
          CollectionDelta::Delta(k, v, p) => {
            let p = p.map(|p| getter(&p)).flatten();
            if let Some(v) = getter(&v) {
              output.push(CollectionDelta::Delta(k, v, p));
            } else if let Some(p) = p {
              output.push(CollectionDelta::Remove(k, p));
            }
          }
          CollectionDelta::Remove(k, p) => {
            if let Some(p) = getter(&p) {
              output.push(CollectionDelta::Remove(k, p));
            }
          }
        }
      }
    }
    let inv_querier = self.relations.access_multi(false);
    if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
      for delta in upstream_changes {
        match delta {
          CollectionDelta::Remove(one, p) => inv_querier(&one, &mut |many| {
            output.push(CollectionDelta::Remove(many, p.clone()));
          }),
          CollectionDelta::Delta(one, change, p) => inv_querier(&one, &mut |many| {
            output.push(CollectionDelta::Delta(many, change.clone(), p.clone()));
          }),
        }
      }
    }

    // todo, check if two change set has overlap and fix delta coherency

    if output.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(output.into_iter()))
    }
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
  fn access(&self, skip_cache: bool) -> impl Fn(&M) -> Option<X> + '_ {
    let upstream_getter = self.upstream.access(skip_cache);
    let access = self.relations.access(skip_cache);
    move |key| {
      let one = access(key)?;
      upstream_getter(&one)
    }
  }

  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = M> + '_ {
    self.relations.iter_key(skip_cache)
  }
}

pub struct ManyToOneReduce<O, M, Upstream, Relation>
where
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveOneToManyRelationship<O, M>,
{
  upstream: Upstream,
  relations: Relation,
  phantom: PhantomData<(O, M)>,
}

impl<O, M, Upstream, Relation> ReactiveCollection<O, ()>
  for ManyToOneReduce<O, M, Upstream, Relation>
where
  M: Clone + 'static,
  O: Clone + 'static,
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveOneToManyRelationship<O, M>,
{
  type Changes = impl Iterator<Item = CollectionDelta<O, ()>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    let relational_changes = self.relations.poll_changes(cx);
    let upstream_changes = self.upstream.poll_changes(cx);

    let mut output = Vec::new(); // it's hard to predict capacity, should we compute it?

    // let getter = self.upstream.access(false);
    // let m_acc = self.relations.access_multi(false);

    // if let Poll::Ready(Some(relational_changes)) = relational_changes {
    //   for change in relational_changes {
    //     let many = change.key().clone();
    //     let new_one = change.value();

    //     if let Some(one_change) = new_one.map(|v| getter(&v)).unwrap() {
    //       output.push(CollectionDelta::Delta(many, one_change));
    //     } else {
    //       output.push(CollectionDelta::Remove(many));
    //     }
    //     //
    //   }
    // }

    // if let Poll::Ready(Some(relational_changes)) = relational_changes {
    //   let getter = self.upstream.access(false);
    //   for change in relational_changes {
    //     let many = change.key().clone();
    //     let new_one = change.value();
    //     if let Some(one_change) = new_one.map(|v| getter(&v)).unwrap() {
    //       output.push(CollectionDelta::Delta(many, one_change));
    //     } else {
    //       output.push(CollectionDelta::Remove(many));
    //     }
    //   }
    // }
    // let inv_querier = self.relations.access_multi(false);
    // if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
    //   for delta in upstream_changes {
    //     match delta {
    //       CollectionDelta::Remove(one) => inv_querier(&one, &mut |many| {
    //         output.push(CollectionDelta::Remove(many));
    //       }),
    //       CollectionDelta::Delta(one, change) => inv_querier(&one, &mut |many| {
    //         output.push(CollectionDelta::Delta(many, change.clone()));
    //       }),
    //     }
    //   }
    // }

    if output.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(output.into_iter()))
    }
  }
}

impl<O, M, Upstream, Relation> VirtualCollection<O, ()>
  for ManyToOneReduce<O, M, Upstream, Relation>
where
  Upstream: ReactiveCollection<M, ()>,
  Relation: ReactiveOneToManyRelationship<O, M>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = O> + '_ {
    self.relations.iter_key_in_multi_collection(skip_cache)
  }

  fn access(&self, skip_cache: bool) -> impl Fn(&O) -> Option<()> + '_ {
    let acc = self.relations.access_multi(skip_cache);
    move |k| {
      let mut has = false;
      acc(k, &mut |_| has = true);
      has.then_some(())
    }
  }
}

pub struct OneToManyRefHashBookKeeping<O, M, T> {
  upstream: T,
  mapping: FastHashMap<O, FastHashSet<M>>,
  /// this could be removed if we redefine the collection change set with previous v
  rev_mapping: FastHashMap<M, Option<O>>,
}

impl<O, M, T> VirtualCollection<M, O> for OneToManyRefHashBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = M> + '_ {
    self.upstream.iter_key(skip_cache)
  }

  fn access(&self, skip_cache: bool) -> impl Fn(&M) -> Option<O> + '_ {
    self.upstream.access(skip_cache)
  }
}

impl<O, M, T> VirtualMultiCollection<O, M> for OneToManyRefHashBookKeeping<O, M, T>
where
  M: Hash + Eq + Clone + 'static,
  O: Hash + Eq + Clone + 'static,
{
  fn iter_key_in_multi_collection(&self, _skip_cache: bool) -> impl Iterator<Item = O> + '_ {
    self.mapping.keys().cloned()
  }

  fn access_multi(&self, _skip_cache: bool) -> impl Fn(&O, &mut dyn FnMut(M)) + '_ {
    move |o, visitor| {
      if let Some(set) = self.mapping.get(o) {
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

    if let Poll::Ready(Some(changes)) = r.clone() {
      for change in changes {
        let mapping = &mut self.mapping;
        let many = change.key().clone();
        let new_one = change.new_value();
        let old_refed_one = self.rev_mapping.get(&many);
        // remove possible old relations
        if let Some(Some(old_refed_one)) = old_refed_one {
          let previous_one_refed_many = mapping.get_mut(old_refed_one).unwrap();
          previous_one_refed_many.remove(&many);
          if previous_one_refed_many.is_empty() {
            mapping.remove(old_refed_one);
            // todo shrink
          }
        }

        // setup new relations
        if let Some(new_one) = &new_one {
          let new_one_refed_many = mapping
            .entry(new_one.clone())
            .or_insert_with(Default::default);
          new_one_refed_many.insert(many.clone());
        }

        self.rev_mapping.insert(many.clone(), new_one);
      }
    }

    r
  }
}

pub struct OneToManyRefDenseBookKeeping<O, M, T> {
  upstream: T,
  mapping_buffer: LinkListPool<u32>,
  mapping: Vec<ListHandle>,
  rev_mapping: Vec<u32>,
  phantom: PhantomData<(O, M)>,
}

impl<O, M, T> VirtualCollection<M, O> for OneToManyRefDenseBookKeeping<O, M, T>
where
  T: ReactiveCollection<M, O>,
{
  fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = M> + '_ {
    self.upstream.iter_key(skip_cache)
  }

  fn access(&self, skip_cache: bool) -> impl Fn(&M) -> Option<O> + '_ {
    self.upstream.access(skip_cache)
  }
}

impl<O, M, T> VirtualMultiCollection<O, M> for OneToManyRefDenseBookKeeping<O, M, T>
where
  M: LinearIdentification + Clone + 'static,
  O: LinearIdentification + Clone + 'static,
{
  fn iter_key_in_multi_collection(&self, _skip_cache: bool) -> impl Iterator<Item = O> + '_ {
    self
      .mapping
      .iter()
      .enumerate()
      .filter_map(|(i, list)| list.is_empty().then_some(O::from_alloc_index(i as u32)))
  }

  fn access_multi(&self, _skip_cache: bool) -> impl Fn(&O, &mut dyn FnMut(M)) + '_ {
    move |o, visitor| {
      if let Some(list) = self.mapping.get(o.alloc_index() as usize) {
        self.mapping_buffer.visit(list, |v, _| {
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

    if let Poll::Ready(Some(changes)) = r.clone() {
      for change in changes {
        let mapping = &mut self.mapping;
        let many = *change.key();
        let new_one = change.new_value();

        let old_refed_one = self.rev_mapping.get(many.alloc_index() as usize);
        // remove possible old relations
        if let Some(old_refed_one) = old_refed_one {
          if *old_refed_one != u32::MAX {
            let previous_one_refed_many = mapping.get_mut(*old_refed_one as usize).unwrap();

            //  this is O(n), should we care about it?
            self
              .mapping_buffer
              .visit_and_remove(previous_one_refed_many, |value, _| {
                let should_remove = *value == many.alloc_index();
                (should_remove, !should_remove)
              });

            if previous_one_refed_many.is_empty() {
              // todo tail shrink
            }
          }
        }

        // setup new relations
        if let Some(new_one) = &new_one {
          mapping[new_one.alloc_index() as usize] = ListHandle::default();
          self.mapping_buffer.insert(
            &mut mapping[new_one.alloc_index() as usize],
            new_one.alloc_index(),
          );
        }

        self.rev_mapping[many.alloc_index() as usize] =
          new_one.map(|v| v.alloc_index()).unwrap_or(u32::MAX)
      }
    }

    r
  }
}
