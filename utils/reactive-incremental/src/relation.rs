use std::{hash::Hash, marker::PhantomData};

use fast_hash_collection::*;
use storage::{LinkListPool, ListHandle};

use crate::*;

// pub trait VirtualMultiCollection<K, V> {
//   fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = K> + '_;
//   fn access_multi(&self, skip_cache: bool) -> impl Fn(&K, &dyn Fn(V)) + '_;
// }
// pub trait ReactiveMultiCollection<K, V>:
//   VirtualMultiCollection<K, V> + Stream + Unpin + 'static
// {
// }

pub trait OneToManyRefBookKeeping<O, M> {
  fn query(&self, many: &M) -> Option<&O>;
  fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M));
  fn iter_many(&self) -> impl Iterator<Item = M> + '_;
  fn apply_change(&mut self, change: CollectionDelta<M, O>);
}

pub trait ReactiveCollectionRelationExt<K, V: IncrementalBase>:
  Sized + 'static + ReactiveCollection<K, V>
{
  fn derive_one_to_many(self) -> OneToManyRefHashBookKeeping<V, K>
  where
    K: Eq,
    V: Eq,
  {
    todo!()
  }

  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn one_to_many_fanout<MK, Relation>(
    self,
    relations: Relation,
  ) -> OneToManyFanout<K, MK, V, Self, Relation>
  where
    V: Clone + Unpin,
    MK: Clone + Unpin,
    K: Clone + Unpin,
    Relation: OneToManyRefBookKeeping<K, MK> + 'static,
  {
    OneToManyFanout {
      upstream: self,
      relations,
      phantom: PhantomData,
    }
  }
}
impl<T, K, V: IncrementalBase> ReactiveCollectionRelationExt<K, V> for T where
  T: Sized + 'static + ReactiveCollection<K, V>
{
}

pub struct OneToManyFanout<O, M, X, Upstream, Relation>
where
  Upstream: ReactiveCollection<O, X>,
  Relation: OneToManyRefBookKeeping<O, M>,
{
  upstream: Upstream,
  relations: Relation,
  phantom: PhantomData<(O, M, X)>,
}

impl<O, M, X, Upstream, Relation> ReactiveCollection<M, X>
  for OneToManyFanout<O, M, X, Upstream, Relation>
where
  M: Clone + Unpin + 'static,
  X: Clone + Unpin + 'static,
  O: Clone + Unpin + 'static,
  Upstream: ReactiveCollection<O, X>,
  Relation: OneToManyRefBookKeeping<O, M> + 'static,
  Relation: Stream<Item = Vec<CollectionDelta<M, O>>> + Unpin,
{
  type Changes = Vec<CollectionDelta<M, X>>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    // We update the relational changes first, note:, this projection is timeline lossy because we
    // assume the consumer will only care about changes happens in the latest reference
    // structure. This is like the flatten signal in single object style.
    let relational_changes = self.relations.poll_next_unpin(cx);
    let upstream_changes = self.upstream.poll_changes(cx);

    let mut output = Vec::new(); // it's hard to predict capacity, should we compute it?
    if let Poll::Ready(Some(relational_changes)) = relational_changes {
      for change in &relational_changes {
        self.relations.apply_change(change.clone());
      }

      let getter = self.upstream.access(false);
      for change in relational_changes {
        let many = change.key().clone();
        let new_one = change.value();
        if let Some(one_change) = new_one.map(|v| getter(&v)).unwrap() {
          output.push(CollectionDelta::Delta(many, one_change));
        } else {
          output.push(CollectionDelta::Remove(many));
        }
      }
    }
    if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
      for delta in upstream_changes {
        match delta {
          CollectionDelta::Remove(one) => self.relations.inv_query(&one, &mut |many| {
            output.push(CollectionDelta::Remove(many.clone()));
          }),
          CollectionDelta::Delta(one, change) => self.relations.inv_query(&one, &mut |many| {
            output.push(CollectionDelta::Delta(many.clone(), change.clone()));
          }),
        }
      }
    }

    if output.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(output))
    }
  }
}

impl<O, M, X, Upstream, Relation> VirtualCollection<M, X>
  for OneToManyFanout<O, M, X, Upstream, Relation>
where
  M: Clone + Unpin,
  X: Clone + Unpin,
  O: Clone + Unpin,
  Upstream: ReactiveCollection<O, X>,
  Relation: OneToManyRefBookKeeping<O, M>,
{
  fn access(&self, skip_cache: bool) -> impl Fn(&M) -> Option<X> + '_ {
    let upstream_getter = self.upstream.access(skip_cache);
    move |key| {
      let one = self.relations.query(key)?;
      upstream_getter(one)
    }
  }

  // skip_cache is always true here
  fn iter_key(&self, _skip_cache: bool) -> impl Iterator<Item = M> + '_ {
    self.relations.iter_many()
  }
}

pub struct OneToManyRefHashBookKeeping<O, M> {
  mapping: FastHashMap<O, FastHashSet<M>>,
  rev_mapping: FastHashMap<M, Option<O>>,
}

impl<O, M> Default for OneToManyRefHashBookKeeping<O, M> {
  fn default() -> Self {
    Self {
      mapping: Default::default(),
      rev_mapping: Default::default(),
    }
  }
}

impl<O, M> OneToManyRefBookKeeping<O, M> for OneToManyRefHashBookKeeping<O, M>
where
  O: Hash + Eq + Clone,
  M: Hash + Eq + Clone,
{
  fn query(&self, many: &M) -> Option<&O> {
    if let Some(r) = self.rev_mapping.get(many) {
      r.as_ref()
    } else {
      None
    }
  }

  fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M)) {
    if let Some(r) = self.mapping.get(one) {
      r.iter().for_each(many_visitor)
    }
  }

  fn iter_many(&self) -> impl Iterator<Item = M> + '_ {
    self.rev_mapping.keys().cloned()
  }

  fn apply_change(&mut self, change: CollectionDelta<M, O>) {
    let mapping = &mut self.mapping;
    let many = change.key().clone();
    let new_one = change.value();
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

pub struct OneToManyRefDenseBookKeeping<O, M> {
  mapping_buffer: LinkListPool<u32>,
  mapping: Vec<ListHandle>,
  rev_mapping: Vec<u32>,
  phantom: PhantomData<(O, M)>,
}

impl<O, M> Default for OneToManyRefDenseBookKeeping<O, M> {
  fn default() -> Self {
    Self {
      mapping_buffer: Default::default(),
      mapping: Default::default(),
      rev_mapping: Default::default(),
      phantom: Default::default(),
    }
  }
}

impl<O, M> OneToManyRefBookKeeping<O, M> for OneToManyRefDenseBookKeeping<O, M>
where
  O: LinearIdentified + Copy,
  M: LinearIdentified + Copy,
{
  fn query(&self, many: &M) -> Option<&O> {
    todo!()
  }

  fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M)) {
    todo!()
  }

  fn iter_many(&self) -> impl Iterator<Item = M> + '_ {
    [].into_iter()
  }

  fn apply_change(&mut self, change: CollectionDelta<M, O>) {
    let mapping = &mut self.mapping;
    let many = change.key();
    let new_one = change.value();

    let old_refed_one = self.rev_mapping.get(many.alloc_index() as usize);
    // remove possible old relations
    if let Some(old_refed_one) = old_refed_one {
      if *old_refed_one != u32::MAX {
        // let previous_one_refed_many = mapping.get_mut(old_refed_one).unwrap();
        // // this is O(n), should we care about it?
        // previous_one_refed_many.remove(&many);
        // if previous_one_refed_many.is_empty() {
        //   mapping.remove(old_refed_one);
        // todo shrink
        // }
      }
    }

    // setup new relations
    if let Some(new_one) = &new_one {
      // let new_one_refed_many = mapping
      //   .entry(new_one.clone())
      //   .or_insert_with(Default::default);
      // new_one_refed_many.insert(many.clone());
    }

    // self.rev_mapping.insert(many.clone(), new_one);
  }
}
