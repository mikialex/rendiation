use std::{hash::Hash, marker::PhantomData};

use fast_hash_collection::*;

use crate::*;

/// O for one, M for many, multiple M reference O;
/// This delta is m's o reference change
#[derive(Clone, Copy)]
pub struct ManyToOneReferenceChange<O, M> {
  pub many: M,
  pub new_one: Option<O>,
}

pub struct OneToManyProjection<O, M, X, Upstream, Relation>
where
  Upstream: ReactiveKVCollection<O, X>,
  Relation: ReactiveOneToManyRefBookKeeping<O, M>,
  X: IncrementalBase,
{
  upstream: Upstream,
  relations: Relation,
  o_ty: PhantomData<O>,
  m_ty: PhantomData<M>,
  x_ty: PhantomData<X>,
}

impl<O, M, X, Upstream, Relation> Stream for OneToManyProjection<O, M, X, Upstream, Relation>
where
  M: Clone + Unpin,
  X: Clone + Unpin + IncrementalBase,
  O: Clone + Unpin,
  Upstream: ReactiveKVCollection<O, X>,
  Relation: ReactiveOneToManyRefBookKeeping<O, M>,
{
  // many maybe not attach to any one.
  // so if upstream relation yield a (m, None(o ref)), we directly map to (m, None(x value))
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

      self.upstream.access(|getter| {
        for ManyToOneReferenceChange { many, new_one } in relational_changes {
          if let Some(one_change) = new_one.map(|one| getter(one.clone())).unwrap() {
            output.push(VirtualKVCollectionDelta::Insert(many, one_change));
          } else {
            output.push(VirtualKVCollectionDelta::Remove(many));
          }
        }
      })
    }
    if let Poll::Ready(Some(upstream_changes)) = upstream_changes {
      for delta in upstream_changes {
        match delta {
          VirtualKVCollectionDelta::Insert(one, new) => {
            self.relations.inv_query(&one, &mut |many| {
              output.push(VirtualKVCollectionDelta::Insert(many.clone(), new.clone()));
            })
          }
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

impl<O, M, X, Upstream, Relation> ReactiveKVCollection<M, X>
  for OneToManyProjection<O, M, X, Upstream, Relation>
where
  M: Clone + Unpin,
  X: Clone + Unpin + IncrementalBase,
  O: Clone + Unpin,
  Upstream: ReactiveKVCollection<O, X>,
  Relation: ReactiveOneToManyRefBookKeeping<O, M>,
{
  fn access(&self, getter: impl FnOnce(&dyn Fn(M) -> Option<X>)) {
    self.upstream.access(move |upstream_getter| {
      getter(&|key| {
        let one = self.relations.query(&key)?;
        upstream_getter(one.clone())
      })
    })
  }
}

pub trait ReactiveKVCollectionRelationExt<K, V: IncrementalBase>:
  Sized + 'static + ReactiveKVCollection<K, V>
{
  fn relational_project<MK, Relation>(self, relations: Relation) -> impl ReactiveKVCollection<MK, V>
  where
    V: Clone + Unpin,
    MK: Clone + Unpin,
    K: Clone + Unpin,
    Relation: ReactiveOneToManyRefBookKeeping<K, MK> + 'static,
  {
    OneToManyProjection {
      upstream: self,
      relations,
      o_ty: PhantomData,
      m_ty: PhantomData,
      x_ty: PhantomData,
    }
  }
}
impl<T, K, V: IncrementalBase> ReactiveKVCollectionRelationExt<K, V> for T where
  T: Sized + 'static + ReactiveKVCollection<K, V>
{
}

pub trait ReactiveOneToManyRefBookKeeping<O, M>:
  Stream<Item = Vec<ManyToOneReferenceChange<O, M>>> + Unpin
{
  fn query(&self, many: &M) -> Option<&O>;
  fn inv_query(&self, one: &O, many_visitor: &mut dyn FnMut(&M));
  fn apply_change(&mut self, change: ManyToOneReferenceChange<O, M>);
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
