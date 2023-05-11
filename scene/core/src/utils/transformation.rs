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
use tree::{CoreTree, TreeCollection, TreeMutation, TreeNodeHandle};
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

pub fn merge_two_tree_deltas<T: IncrementalBase>(
  tree_a: impl Stream<Item = TreeMutation<T>>,
  tree_b: impl Stream<Item = TreeMutation<T>>,
) -> impl Stream<Item = TreeMutation<T>> {
  let merged_tree = Default::default();

  futures::stream::select(
    remapping_tree_stream(tree_a, &merged_tree),
    remapping_tree_stream(tree_b, &merged_tree),
  )
}

pub fn remapping_tree_stream<T: IncrementalBase>(
  s: impl Stream<Item = TreeMutation<T>>,
  target: &Arc<RwLock<TreeCollection<()>>>,
) -> impl Stream<Item = TreeMutation<T>> {
  let target = target.clone();
  let mut remapping: HashMap<usize, TreeNodeHandle<()>> = Default::default();
  s.map(move |delta| match delta {
    TreeMutation::Create { data, node } => {
      let handle = target.write().unwrap().create_node(());
      remapping.insert(node, handle);
      TreeMutation::Create {
        data,
        node: handle.index(),
      }
    }
    TreeMutation::Delete(idx) => {
      let handle = remapping.remove(&idx).unwrap();
      target.write().unwrap().delete_node(handle);
      TreeMutation::Delete(handle.index())
    }
    TreeMutation::Mutate { node, delta } => TreeMutation::Mutate {
      node: remapping.get(&node).unwrap().index(),
      delta,
    },
    TreeMutation::Attach {
      parent_target,
      node,
    } => TreeMutation::Attach {
      parent_target: remapping.get(&parent_target).unwrap().index(),
      node: remapping.get(&node).unwrap().index(),
    },
    TreeMutation::Detach { node } => TreeMutation::Detach {
      node: remapping.get(&node).unwrap().index(),
    },
  })
}
