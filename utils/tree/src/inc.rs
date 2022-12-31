use crate::*;
use ::incremental::*;

pub enum SharedTreeMutation<T: IncrementalBase> {
  Create(NodeRef<T>),
  Delete(TreeNodeHandle<T>),
  Mutate {
    node: TreeNodeHandle<T>,
    delta: T::Delta,
  },
  Attach {
    parent_target: TreeNodeHandle<T>,
    node: TreeNodeHandle<T>,
  },
  Detach {
    node: TreeNodeHandle<T>,
  },
}

impl<T: IncrementalBase> Clone for SharedTreeMutation<T> {
  fn clone(&self) -> Self {
    match self {
      SharedTreeMutation::Create(n) => SharedTreeMutation::Create(n.clone()),
      SharedTreeMutation::Delete(n) => SharedTreeMutation::Delete(n.clone()),
      SharedTreeMutation::Mutate { node, delta } => SharedTreeMutation::Mutate {
        node: node.clone(),
        delta: delta.clone(),
      },
      SharedTreeMutation::Attach {
        parent_target,
        node,
      } => SharedTreeMutation::Attach {
        parent_target: parent_target.clone(),
        node: node.clone(),
      },
      SharedTreeMutation::Detach { node } => SharedTreeMutation::Detach { node: node.clone() },
    }
  }
}

impl<T: IncrementalBase + Send + Sync> IncrementalBase for SharedTreeCollection<T> {
  type Delta = SharedTreeMutation<T>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    let tree = self.inner.write().unwrap();
    for (handle, node) in &tree.nodes.data {
      if node.first_child.is_none() {
        let node = tree.create_node_ref(handle);
        // todo fix traverse_pair skip leaf/parent node
        node.traverse_pair(&mut |self_node, parent| {
          cb(SharedTreeMutation::Create(NodeRef {
            nodes: self.clone(),
            handle: self_node.node.handle(),
          }));
          cb(SharedTreeMutation::Attach {
            parent_target: parent.node.handle(),
            node: self_node.node.handle(),
          });
        })
      }
    }
  }
}

// impl<T: ApplicableIncremental + Clone + Send + Sync> ApplicableIncremental
//   for SharedTreeCollection<T>
// {
//   type Error = ();

//   fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
//     let tree = self.inner.write().unwrap();
//     match delta {
//       SharedTreeMutation::Create(d) => {
//         tree.create_node(d);
//       }
//       SharedTreeMutation::Delete(d) => tree.delete_node(d),
//       SharedTreeMutation::Mutate { node, delta } => {
//         let node = tree.get_node_mut(node).data_mut();
//         node.apply(delta).unwrap();
//       }
//       SharedTreeMutation::Attach {
//         parent_target,
//         node,
//       } => tree.node_add_child_by(parent_target, node).unwrap(),
//       SharedTreeMutation::Detach { node } => {
//         tree.node_detach_parent(node).unwrap();
//       }
//     }
//     Ok(())
//   }
// }

// impl<T> IncrementalMutatorHelper for TreeCollection<T>
// where
//   Self: IncrementalBase,
//   T: IncrementalBase + Clone,
// {
//   type Mutator<'a> = TreeCollectionReactiveMutator<'a, T>
//   where
//     Self: 'a;

//   fn create_mutator<'a>(
//     &'a mut self,
//     collector: &'a mut dyn FnMut(Self::Delta),
//   ) -> Self::Mutator<'a> {
//     TreeCollectionReactiveMutator {
//       inner: self,
//       collector,
//     }
//   }
// }

// pub struct MutateMapper<'a, T: IncrementalBase, C: FnMut(T::Delta) + 'a> {
//   inner: &'a mut T,
//   collector: C,
// }

// impl<'a, T, C> MutateMapper<'a, T, C>
// where
//   T: IncrementalMutatorHelper,
//   C: FnMut(T::Delta) + 'a,
// {
//   pub fn mutate(&'a mut self) -> T::Mutator<'a> {
//     self.inner.create_mutator(&mut self.collector)
//   }
// }

// pub struct TreeCollectionReactiveMutator<'a, T: IncrementalBase + Clone + Send + Sync> {
//   inner: &'a mut SharedTreeCollection<T>,
//   collector: &'a mut dyn FnMut(DeltaOf<SharedTreeCollection<T>>),
// }

// impl<'a, T: IncrementalBase + Clone + Send + Sync> TreeCollectionReactiveMutator<'a, T> {
//   pub fn create(&mut self, node: T) -> TreeNodeHandle<T> {
//     (self.collector)(SharedTreeMutation::Create(node.clone()));
//     self.inner.create_node(node)
//   }

//   pub fn get_node_mut(
//     &'a mut self,
//     node: TreeNodeHandle<T>,
//   ) -> MutateMapper<'a, T, impl FnMut(T::Delta) + 'a> {
//     let t = self.inner.get_node_mut(node).data_mut();
//     let collector = &mut self.collector;
//     let collector =
//       move |delta| collector(DeltaOf::<SharedTreeCollection<T>>::Mutate { node, delta });

//     MutateMapper {
//       inner: t,
//       collector,
//     }
//   }
// }

// #[test]
// fn test_nested_mutator() {
//   let mut tree = TreeCollection::<usize>::default();
//   let mut tree_collector = |_| {};
//   let root = {
//     let mut tree_mutator = tree.create_mutator(&mut tree_collector);
//     let root = tree_mutator.create(1);
//     let mut node_mutation_mapper = tree_mutator.get_node_mut(root);
//     let mut node_mutator = node_mutation_mapper.mutate();
//     node_mutator.apply(2);
//     root
//   };

//   let tree_root_raw = tree.get_node(root).data();
//   assert_eq!(*tree_root_raw, 2);
// }
