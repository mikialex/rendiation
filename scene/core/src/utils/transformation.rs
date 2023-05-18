use std::collections::HashSet;

use crate::*;

pub enum IndependentItemContainerDelta<K, T: IncrementalBase> {
  Remove(K),
  Insert(K, T),
  Mutate(K, DeltaOf<T>),
}

pub enum ContainerRefRetainDelta<K, T> {
  Remove(K),
  Insert(K, T),
}

pub enum ContainerRefRetainContentDelta<T> {
  Remove(T),
  Insert(T),
}

impl<T: IncrementalBase> From<ArenaDelta<T>>
  for IndependentItemContainerDelta<arena::Handle<T>, T>
{
  fn from(value: ArenaDelta<T>) -> Self {
    match value {
      ArenaDelta::Mutate((delta, handle)) => IndependentItemContainerDelta::Mutate(handle, delta),
      ArenaDelta::Insert((data, handle)) => IndependentItemContainerDelta::Insert(handle, data),
      ArenaDelta::Remove(handle) => IndependentItemContainerDelta::Remove(handle),
    }
  }
}

use futures::*;
use tree::{CoreTree, TreeMutation, TreeNodeHandle};
pub trait IncrementalStreamTransform {
  fn transform_ref_retained_to_ref_retained_content_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainContentDelta<T>>
  where
    Self: Stream<Item = ContainerRefRetainDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static;

  fn transform_delta_to_ref_retained_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainDelta<K, T>>
  where
    Self: Stream<Item = IndependentItemContainerDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static + IncrementalBase<Delta = T>;
}

impl<X> IncrementalStreamTransform for X {
  fn transform_ref_retained_to_ref_retained_content_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainContentDelta<T>>
  where
    Self: Stream<Item = ContainerRefRetainDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static,
  {
    let mut cache: HashMap<K, T> = HashMap::new();
    self.map(move |v| match v {
      ContainerRefRetainDelta::Remove(key) => {
        let value = cache
          .remove(&key)
          .expect("failed to retrieve source value in retained content transformation");
        ContainerRefRetainContentDelta::Remove(value)
      }
      ContainerRefRetainDelta::Insert(k, v) => {
        cache.insert(k, v.clone());
        ContainerRefRetainContentDelta::Insert(v)
      }
    })
  }

  fn transform_delta_to_ref_retained_by_hashing<K, T>(
    self,
  ) -> impl Stream<Item = ContainerRefRetainDelta<K, T>>
  where
    Self: Stream<Item = IndependentItemContainerDelta<K, T>>,
    K: Hash + Eq + Clone + 'static,
    T: Clone + 'static + IncrementalBase<Delta = T>,
  {
    let mut cache: HashSet<K> = HashSet::new();
    self
      .map(move |v| {
        // this one will always stay on stack
        let mut one_or_two = smallvec::SmallVec::<[_; 2]>::default();
        match v {
          IndependentItemContainerDelta::Remove(key) => {
            assert!(cache.remove(&key));
            one_or_two.push(ContainerRefRetainDelta::Remove(key));
          }
          IndependentItemContainerDelta::Insert(key, value) => {
            assert!(cache.insert(key.clone()));
            one_or_two.push(ContainerRefRetainDelta::Insert(key, value));
          }
          IndependentItemContainerDelta::Mutate(key, value) => {
            if cache.remove(&key) {
              one_or_two.push(ContainerRefRetainDelta::Remove(key.clone()));
            }
            one_or_two.push(ContainerRefRetainDelta::Insert(key, value));
          }
        };
        futures::stream::iter(one_or_two)
      })
      .flatten()
  }
}

// pub trait LinearKey {
//   fn get_index(&self) -> usize;
// }

// impl<T> LinearKey for arena::Handle<T> {
//   fn get_index(&self) -> usize {
//     self.index()
//   }
// }

pub trait ArenaDeltaStreamTransform {
  fn transform_ref_retained_content_to_arena_by_hashing<T>(
    self,
  ) -> impl Stream<Item = ArenaDelta<T>>
  where
    Self: Stream<Item = ContainerRefRetainContentDelta<T>>,
    T: IncrementalBase + GlobalIdentified;
}
impl<X> ArenaDeltaStreamTransform for X {
  fn transform_ref_retained_content_to_arena_by_hashing<T>(
    self,
  ) -> impl Stream<Item = ArenaDelta<T>>
  where
    Self: Stream<Item = ContainerRefRetainContentDelta<T>>,
    T: IncrementalBase + GlobalIdentified,
  {
    let mut output_arena = arena::Arena::<()>::new();
    let mut output_remapping: HashMap<usize, arena::Handle<()>> = Default::default();
    self.map(move |item| match item {
      ContainerRefRetainContentDelta::Insert(item) => {
        let handle = output_arena.insert(());
        output_remapping.insert(item.guid(), handle);
        ArenaDelta::Insert((item, unsafe { handle.cast_type() }))
      }
      ContainerRefRetainContentDelta::Remove(item) => {
        let handle = output_remapping.remove(&item.guid()).unwrap();
        output_arena.remove(handle).unwrap();
        let handle = unsafe { handle.cast_type() };
        ArenaDelta::Remove(handle)
      }
    })
  }
}

pub fn merge_two_tree_deltas(
  tree_a: impl Stream<Item = TreeMutation<SceneNodeDataImpl>>,
  tree_b: impl Stream<Item = TreeMutation<SceneNodeDataImpl>>,
) -> (
  impl Stream<Item = TreeMutation<SceneNodeDataImpl>>,
  impl Fn(&SceneNode) -> SceneNode + Clone,
  impl Fn(&SceneNode) -> SceneNode + Clone,
) {
  let merged_tree = Default::default();

  let (s_a, m_a) = remapping_tree_stream(tree_a, &merged_tree);
  let (s_b, m_b) = remapping_tree_stream(tree_b, &merged_tree);
  (futures::stream::select(s_a, s_b), m_a, m_b)
}

pub fn recreate_tree_nodes(
  delta: TreeMutation<SceneNodeDataImpl>,
  target: &SceneNodeCollection,
  holder: &mut HashMap<usize, SceneNode>,
) {
  match delta {
    TreeMutation::Create { data, node } => {
      let handle = target
        .inner
        .inner
        .write()
        .unwrap()
        .create_node(Identity::new(data));
      let r_node = target.create_node_at(handle);
      holder.insert(node, r_node);
    }
    TreeMutation::Delete(idx) => {
      holder.remove(&idx);
      // node dropper will do the cleanup, will it?
    }
    TreeMutation::Mutate { node, delta } => holder
      .get(&node)
      .unwrap()
      .mutate(|mut node| node.modify(delta)),
    TreeMutation::Attach {
      parent_target,
      node,
    } => {
      let parent = holder.get(&parent_target).unwrap();
      let node = holder.get(&node).unwrap();
      node
        .inner // todo, for god's sake we add pub to upstream crate structs for supporting this!
        .inner
        .write()
        .unwrap()
        .attach_to(&parent.inner.inner.read().unwrap());
    }
    TreeMutation::Detach { node } => {
      let node = holder.get(&node).unwrap();
      node.inner.inner.write().unwrap().detach_from_parent();
    }
  }
}

pub fn scene_folding(
  s: impl Stream<Item = SceneInnerDelta>,
) -> (impl Stream<Item = ()>, (Scene, SceneNodeDeriveSystem)) {
  let (scene, d_sys) = SceneInner::new();
  let scene_c = scene.clone();

  let mut scene_node_holder = HashMap::<usize, SceneNode>::new();

  let folder = s.map(move |mut delta| {
    scene.mutate(|mut scene| {
      transform_scene_delta_node(&mut delta, |node: &SceneNode| {
        scene_node_holder
          .get(&node.raw_handle().index())
          .unwrap()
          .clone()
      });

      if let SceneInnerDelta::nodes(delta) = &delta {
        recreate_tree_nodes(delta.clone(), &scene.nodes, &mut scene_node_holder);
      }

      scene.modify(delta);
    });
  });

  (folder, (scene_c, d_sys))
}

pub fn remapping_tree_stream(
  s: impl Stream<Item = TreeMutation<SceneNodeDataImpl>>,
  target: &SceneNodeCollection,
) -> (
  impl Stream<Item = TreeMutation<SceneNodeDataImpl>>,
  impl Fn(&SceneNode) -> SceneNode + Clone,
) {
  let target = target.clone();
  let target_c = target.clone();
  let remapping: HashMap<usize, (TreeNodeHandle<SceneNodeData>, SceneNode)> = Default::default();
  let remapping = Arc::new(RwLock::new(remapping));
  let remapping_c = remapping.clone();
  let stream = s.map(move |delta| {
    let mut remapping = remapping.write().unwrap();
    match delta {
      TreeMutation::Create { data, node } => {
        let handle = target
          .inner
          .inner
          .write()
          .unwrap()
          .create_node(Identity::new(data.clone()));
        let r_node = target.create_node_at(handle);
        remapping.insert(node, (handle, r_node));
        TreeMutation::Create {
          data,
          node: handle.index(),
        }
      }
      TreeMutation::Delete(idx) => {
        let (handle, r_node) = remapping.remove(&idx).unwrap();
        std::mem::ManuallyDrop::new(r_node);
        target.inner.inner.write().unwrap().delete_node(handle);
        TreeMutation::Delete(handle.index())
      }
      TreeMutation::Mutate { node, delta } => TreeMutation::Mutate {
        node: remapping.get(&node).unwrap().0.index(),
        delta,
      },
      TreeMutation::Attach {
        parent_target,
        node,
      } => TreeMutation::Attach {
        parent_target: remapping.get(&parent_target).unwrap().0.index(),
        node: remapping.get(&node).unwrap().0.index(),
      },
      TreeMutation::Detach { node } => TreeMutation::Detach {
        node: remapping.get(&node).unwrap().0.index(),
      },
    }
  });

  let mapper = move |node: &SceneNode| {
    remapping_c
      .read()
      .unwrap()
      .get(&node.raw_handle().index())
      .unwrap()
      .1
      .clone()
  };

  (stream, mapper)
}
