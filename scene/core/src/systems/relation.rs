use core::hash::Hash;
use std::{
  pin::Pin,
  task::{Context, Poll},
};

use arena::ArenaDelta;
use futures::StreamExt;
use reactive::{SignalStreamExt, VecUpdateUnit};
use tree::TreeMutation;

use crate::*;

pub struct OneToManyRefBookKeeping<O, M> {
  // we could use more efficient data structure
  mapping: FastHashMap<O, FastHashSet<M>>,
}

impl<O, M> Default for OneToManyRefBookKeeping<O, M> {
  fn default() -> Self {
    Self {
      mapping: Default::default(),
    }
  }
}

pub enum OneToManyRelationChange<O, M> {
  CreateOne(O),
  RemoveOne(O),
  OneRefedByMany(O, M),
  OneDeRefedByMany(O, M),
}

impl<O, M> OneToManyRefBookKeeping<O, M>
where
  O: Hash + Eq,
  M: Hash + Eq,
{
  pub fn apply_change(&mut self, change: OneToManyRelationChange<O, M>) {
    let mapping = &mut self.mapping;
    match change {
      OneToManyRelationChange::CreateOne(one) => {
        debug_assert!(!mapping.contains_key(&one));
        mapping.insert(one, Default::default());
      }
      OneToManyRelationChange::RemoveOne(one) => {
        debug_assert!(mapping.contains_key(&one));
        debug_assert!(mapping.get(&one).unwrap().is_empty());
        mapping.remove(&one);
      }
      OneToManyRelationChange::OneRefedByMany(one, many) => {
        let inner = mapping.get_mut(&one).unwrap();
        debug_assert!(!inner.contains(&many));
        inner.insert(many);
      }
      OneToManyRelationChange::OneDeRefedByMany(one, many) => {
        let inner = mapping.get_mut(&one).unwrap();
        debug_assert!(inner.contains(&many));
        inner.remove(&many);
      }
    };
  }
}

/// the delta type not contains the old state before the mutation, so we have to keep the state by
/// ourself
pub enum OneToManyRelationChangeFull<O, M> {
  CreateOne(O),
  RemoveOne(O),
  OneRefedByMany(O, M),
  OneDeRefedByMany(O, M),
  OneMutateMany(O, M),
  RemoveMany(M),
}

impl<O, M> OneToManyRelationChangeFull<O, M>
where
  O: Hash + Eq + Clone,
  M: Hash + Eq + Clone,
{
  fn normalize(
    self,
    states: &mut FastHashMap<M, O>,
    mut cb: impl FnMut(OneToManyRelationChange<O, M>),
  ) {
    use OneToManyRelationChange as O;
    match self {
      Self::CreateOne(one) => cb(O::CreateOne(one)),
      Self::RemoveOne(one) => cb(O::RemoveOne(one)),
      Self::OneRefedByMany(one, many) => {
        debug_assert!(!states.contains_key(&many));
        states.insert(many.clone(), one.clone());
        cb(O::OneRefedByMany(one, many))
      }
      Self::OneDeRefedByMany(one, many) => {
        debug_assert!(states.contains_key(&many));
        states.remove(&many);
        cb(O::OneDeRefedByMany(one, many))
      }
      Self::OneMutateMany(one, many) => {
        if let Some(one_before_mutate) = states.remove(&many) {
          cb(O::OneDeRefedByMany(one_before_mutate, many.clone()));
        }
        cb(O::OneRefedByMany(one, many));
      }
      Self::RemoveMany(many) => {
        if let Some(one_before_mutate) = states.remove(&many) {
          cb(O::OneDeRefedByMany(one_before_mutate, many));
        }
      }
    }
  }
}

use OneToManyRelationChangeFull as Change;

pub struct NodeReferenceModelBookKeeping {
  inner: Arc<RwLock<OneToManyRefBookKeeping<usize, usize>>>,
  source: NodeReferenceModelBookKeepingSource,
}

impl Stream for NodeReferenceModelBookKeeping {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.source.poll_next_unpin(cx)
  }
}

type NodeReferenceModelBookKeepingSource = impl Stream<Item = ()> + Unpin;

impl NodeReferenceModelBookKeeping {
  pub fn query_node_referenced_model_indices(&self, node: &SceneNode, model_index: impl Fn(usize)) {
    let inner = self.inner.read().unwrap();
    if let Some(models) = inner.mapping.get(&node.raw_handle().index()) {
      models.iter().copied().for_each(model_index)
    }
  }

  pub fn new(scene: &SceneCore) -> Self {
    let source1 = scene
      .unbound_listen_by(move |v, send| match v {
        MaybeDeltaRef::Delta(d) => match d {
          SceneInternalDelta::models(delta) => on_model_mutate(send, delta),
          SceneInternalDelta::nodes(delta) => on_tree_mutate(send, delta),
          _ => {}
        },
        MaybeDeltaRef::All(scene) => {
          scene.nodes.expand(|delta| on_tree_mutate(send, &delta));
          scene.models.expand(|delta| on_model_mutate(send, &delta));
        }
      })
      .batch_processing();

    use arena::ArenaDelta::*;
    let source2 = scene
      .unbound_listen_by(|view, send| match view {
        MaybeDeltaRef::All(scene) => scene.models.expand(send),
        MaybeDeltaRef::Delta(delta) => {
          if let SceneInternalDelta::models(model_delta) = delta {
            send(model_delta.clone())
          }
        }
      })
      .map(move |model_delta| match model_delta {
        Mutate((new, handle)) => (handle.index(), Some(build_stream(&new, handle.index()))),
        Insert((new, handle)) => (handle.index(), Some(build_stream(&new, handle.index()))),
        Remove(handle) => (handle.index(), None),
      })
      .flatten_into_vec_stream_signal()
      .filter_map_sync(|d| {
        if let VecUpdateUnit::Updates(updates) = d {
          Some(updates.into_iter().map(|c| c.item).collect::<Vec<_>>())
        } else {
          None
        }
      });

    let inner: Arc<RwLock<OneToManyRefBookKeeping<usize, usize>>> = Default::default();
    let current_relation: Arc<RwLock<FastHashMap<usize, usize>>> = Default::default();

    let inner_c = inner.clone();

    // should always consume watched delta changes first, or we will have message order issue
    // todo investigate better ways to solve order issue, because early drop inner change is better
    // for performance
    let source = futures::stream::select_with_strategy(source1, source2, |_: &mut ()| {
      futures::stream::PollNext::Right
    })
    .map(move |deltas| {
      let mut states = current_relation.write().unwrap();
      let mut inner = inner_c.write().unwrap();
      for delta in deltas {
        delta.normalize(&mut states, |normalized| {
          inner.apply_change(normalized);
        });
      }
    });

    Self { inner, source }
  }
}

fn on_tree_mutate(send: impl Fn(Change<usize, usize>), delta: &TreeMutation<SceneNodeData>) {
  match delta {
    tree::TreeMutation::Create { node, .. } => send(Change::CreateOne(*node)),
    tree::TreeMutation::Delete(node) => send(Change::RemoveOne(*node)),
    _ => {}
  }
}

fn on_model_mutate(
  send: impl Fn(Change<usize, usize>) + Copy,
  delta: &ArenaDelta<IncrementalSignalPtr<SceneModelImpl>>,
) {
  match delta {
    arena::ArenaDelta::Mutate((model, h)) => {
      on_model_mutate(send, &arena::ArenaDelta::Remove(*h));
      on_model_mutate(send, &arena::ArenaDelta::Insert((model.clone(), *h)));
    }
    arena::ArenaDelta::Insert((model, h)) => {
      let node = model.read().node.raw_handle().index();
      send(Change::OneRefedByMany(node, h.index()));
    }
    arena::ArenaDelta::Remove(h) => send(Change::RemoveMany(h.index())),
  }
}

fn build_stream(
  model: &SceneModel,
  model_index: usize,
) -> impl Stream<Item = Change<usize, usize>> {
  model.unbound_listen_by(move |v, send| match v {
    MaybeDeltaRef::Delta(d) => {
      if let SceneModelImplDelta::node(node) = d {
        send(Change::OneMutateMany(
          node.raw_handle().index(),
          model_index,
        ))
      }
    }
    MaybeDeltaRef::All(_) => {
      // this is covered by arena insert. we do not trigger here
    }
  })
}
