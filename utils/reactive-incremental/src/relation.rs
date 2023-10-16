use std::hash::Hash;

use fast_hash_collection::*;

use crate::*;

/// O for one, M for many, multiple M reference O;
/// This delta is m's o reference change
pub struct ManyToOneReferenceChange<O, M> {
  pub many: M,
  pub new_one: Option<O>,
}

pub struct OneToManyProjection<O, M, X> {
  upstream: Box<dyn ReactiveKVCollection<O, X>>,
  relations: Box<dyn ReactiveOneToManyRefBookKeeping<O, M>>,
}

impl<O, M, X> Stream for OneToManyProjection<O, M, X>
where
  M: Clone,
  X: Clone,
{
  // many maybe not attach to any one.
  // so if upstream relation yield a (m, None(o ref)), we directly map to (m, None(x value))
  type Item = Vec<(M, Option<X>)>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    // We update the relational changes first, note:, this projection is timeline lossy because we
    // assume the consumer will only care about changes happens in the latest reference
    // structure. This is like the flatten signal in single object style.
    let relational_changes = self.relations.poll_next_unpin(cx);
    let upstream_changes = self.upstream.poll_next_unpin(cx);

    let mut output = Vec::new(); // it's hard to predict capacity, should we compute it?
    if let Poll::Ready(Some(relational_changes)) = relational_changes {
      for ManyToOneReferenceChange { many, new_one } in relational_changes {
        let one_change = new_one.map(|one| self.upstream.access(&one)).unwrap();
        output.push((many, one_change));
      }
    }
    if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
      for (one, change) in upstream_changes {
        self.relations.inv_query(&one, &mut |many| {
          output.push((many.clone(), change.clone()));
        })
      }
    }

    if output.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(output))
    }
  }
}

impl<O, M, X> ReactiveKVCollection<M, X> for OneToManyProjection<O, M, X>
where
  M: Clone,
  X: Clone,
{
  fn access(&self, key: &M) -> Option<X> {
    let one = self.relations.query(key)?;
    self.upstream.access(one)
  }
}

pub trait ReactiveKVCollectionExt<K, V>: Sized + 'static + ReactiveKVCollection<K, V> {
  fn relational_project<MK>(
    self,
    relations: impl ReactiveOneToManyRefBookKeeping<K, MK> + 'static,
  ) -> impl ReactiveKVCollection<MK, V>
  where
    V: Clone,
    MK: Clone,
  {
    OneToManyProjection {
      upstream: Box::new(self),
      relations: Box::new(relations),
    }
  }
  // fn map<V2>(self, f: impl Fn(V) -> V2) -> impl ReactiveKVCollection<K, V2> {
  //   //
  // }
  // fn zip<V2>(
  //   self,
  //   other: impl ReactiveKVCollection<K, V2>,
  // ) -> impl ReactiveKVCollection<K, (V, V2)> {
  //   //
  // }
}
impl<T, K, V> ReactiveKVCollectionExt<K, V> for T where
  T: Sized + 'static + ReactiveKVCollection<K, V>
{
}

pub trait ReactiveKVCollection<K, V>: Stream<Item = Vec<(K, Option<V>)>> + Unpin {
  /// should access after poll
  fn access(&self, key: &K) -> Option<V>;
}

pub trait ReactiveOneToManyRefBookKeeping<O, M>:
  Stream<Item = Vec<ManyToOneReferenceChange<O, M>>> + Unpin
{
  fn query(&self, many: &M) -> Option<&O>;
  fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M));
}

// let sm_local =  local_bbox
//   .relational_project(mesh_model_ref)
//   .relational_project(model_sm_ref)

// let sm_world_mat = node_mat
// .relational_project(node_sm_ref)

// let sm_world = sm_world_mat.merge(sm_local)

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

impl<O, M> OneToManyRefHashBookKeeping<O, M>
where
  O: Hash + Eq + Clone,
  M: Hash + Eq + Clone,
{
  pub fn apply_change(&mut self, change: ManyToOneReferenceChange<O, M>) {
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
