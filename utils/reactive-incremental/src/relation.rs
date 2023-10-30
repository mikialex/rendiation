use std::{hash::Hash, marker::PhantomData};

use fast_hash_collection::*;
use storage::{LinkListPool, ListHandle};

use crate::*;

// pub trait OneToOneReactiveRelation<A, B>:
//   ReactiveKVCollection<A, B> + ReactiveKVCollection<B, A>
// {
// }

// pub trait OneToManyReactiveRelation<O, M>: ReactiveKVCollection<M, O>
// {
//   fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M));
// }

/// O for one, M for many, multiple M reference O;
/// This delta is m's o reference change
#[derive(Clone, Copy)]
pub struct ManyToOneReferenceChange<O, M> {
  pub many: M,
  pub new_one: Option<O>,
}

impl<O, M> VirtualKVCollectionDelta<M, Option<O>> {
  /// not make sense sometimes
  pub fn into_ref_change(self) -> ManyToOneReferenceChange<O, M> {
    match self {
      VirtualKVCollectionDelta::Delta(many, one) => ManyToOneReferenceChange { many, new_one: one },
      VirtualKVCollectionDelta::Remove(many) => ManyToOneReferenceChange {
        many,
        new_one: None,
      },
    }
  }
}

pub struct OneToManyProjection<O, M, X, Upstream, Relation>
where
  Upstream: ReactiveKVCollection<O, X>,
  Upstream::Item: IntoIterator<Item = VirtualKVCollectionDelta<O, X>>,
  Relation: OneToManyRefBookKeeping<O, M>,
  X: IncrementalBase,
{
  upstream: Upstream,
  relations: Relation,
  phantom: PhantomData<(O, M, X)>,
}

impl<O, M, X, Upstream, Relation> Stream for OneToManyProjection<O, M, X, Upstream, Relation>
where
  M: Clone + Unpin,
  X: Clone + Unpin + IncrementalBase,
  O: Clone + Unpin,
  Upstream: ReactiveKVCollection<O, X>,
  Upstream::Item: IntoIterator<Item = VirtualKVCollectionDelta<O, X>>,
  Relation: OneToManyRefBookKeeping<O, M>,
  Relation: Stream<Item = Vec<ManyToOneReferenceChange<O, M>>> + Unpin,
{
  type Item = Vec<VirtualKVCollectionDelta<M, X>>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    // We update the relational changes first, note:, this projection is timeline lossy because we
    // assume the consumer will only care about changes happens in the latest reference
    // structure. This is like the flatten signal in single object style.
    let relational_changes = self.relations.poll_next_unpin(cx);
    let upstream_changes = self.upstream.poll_next_unpin(cx);

    let mut output = Vec::new(); // it's hard to predict capacity, should we compute it?
    if let Poll::Ready(Some(relational_changes)) = relational_changes {
      for change in &relational_changes {
        self.relations.apply_change(change.clone());
      }

      let getter = self.upstream.access(false);
      for ManyToOneReferenceChange { many, new_one } in relational_changes {
        if let Some(one_change) = new_one.map(|v| getter(&v)).unwrap() {
          output.push(VirtualKVCollectionDelta::Delta(many, one_change));
        } else {
          output.push(VirtualKVCollectionDelta::Remove(many));
        }
      }
    }
    if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
      for delta in upstream_changes {
        match delta {
          VirtualKVCollectionDelta::Remove(one) => self.relations.inv_query(&one, &mut |many| {
            output.push(VirtualKVCollectionDelta::Remove(many.clone()));
          }),
          VirtualKVCollectionDelta::Delta(one, change) => {
            self.relations.inv_query(&one, &mut |many| {
              output.push(VirtualKVCollectionDelta::Delta(
                many.clone(),
                change.clone(),
              ));
            })
          }
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

impl<O, M, X, Upstream, Relation> VirtualKVCollection<M, X>
  for OneToManyProjection<O, M, X, Upstream, Relation>
where
  M: Clone + Unpin,
  X: Clone + Unpin + IncrementalBase,
  O: Clone + Unpin,
  Upstream: ReactiveKVCollection<O, X>,
  Upstream::Item: IntoIterator<Item = VirtualKVCollectionDelta<O, X>>,
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

pub trait ReactiveKVCollectionRelationExt<K, V: IncrementalBase>:
  Sized + 'static + ReactiveKVCollection<K, V>
where
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
  /// project map<O, V> -> map<M, V> when we have O - M one to many
  fn relational_project<MK, Relation>(
    self,
    relations: Relation,
  ) -> OneToManyProjection<K, MK, V, Self, Relation>
  where
    V: Clone + Unpin,
    MK: Clone + Unpin,
    K: Clone + Unpin,
    Relation: OneToManyRefBookKeeping<K, MK> + 'static,
  {
    OneToManyProjection {
      upstream: self,
      relations,
      phantom: PhantomData,
    }
  }
}
impl<T, K, V: IncrementalBase> ReactiveKVCollectionRelationExt<K, V> for T
where
  T: Sized + 'static + ReactiveKVCollection<K, V>,
  Self::Item: IntoIterator<Item = VirtualKVCollectionDelta<K, V>>,
{
}

pub trait OneToManyRefBookKeeping<O, M> {
  fn query(&self, many: &M) -> Option<&O>;
  fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M));
  fn iter_many(&self) -> impl Iterator<Item = M> + '_;
  fn apply_change(&mut self, change: ManyToOneReferenceChange<O, M>);
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

  fn apply_change(&mut self, change: ManyToOneReferenceChange<O, M>) {
    let mapping = &mut self.mapping;
    let ManyToOneReferenceChange { many, new_one } = change;
    let old_refed_one = self.rev_mapping.get(&many);
    // remove possible old relations
    if let Some(Some(old_refed_one)) = old_refed_one {
      let previous_one_refed_many = mapping.get_mut(old_refed_one).unwrap();
      previous_one_refed_many.remove(&many);
      if previous_one_refed_many.is_empty() {
        mapping.remove(old_refed_one);
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

impl<O, M> OneToManyRefBookKeeping<O, M> for OneToManyRefDenseBookKeeping<O, M> {
  fn query(&self, many: &M) -> Option<&O> {
    todo!()
  }

  fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M)) {
    todo!()
  }

  fn iter_many(&self) -> impl Iterator<Item = M> + '_ {
    [].into_iter()
  }

  fn apply_change(&mut self, change: ManyToOneReferenceChange<O, M>) {
    let ManyToOneReferenceChange { many, new_one } = change;
    todo!()
  }
}
