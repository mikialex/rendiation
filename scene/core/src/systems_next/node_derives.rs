// use tree::*;

// use crate::*;

// pub struct NodeIncrementalDeriveSystem {
//   inner:
//     TreeHierarchyDerivedSystem<SceneNodeDerivedData,
// ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>>, }

// impl NodeIncrementalDeriveSystem {
//   pub fn new(nodes: &SceneNodeCollection) -> Self {
//     let stream = nodes.inner.source.batch_listen();
//     let inner = TreeHierarchyDerivedSystem::<
//       SceneNodeDerivedData,
//       ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>,
//     >::new::<ParentTree, _, _, _>(stream, &nodes.inner);
//     Self { inner }
//   }
// }

// // impl VirtualCollection<u64, SceneNodeDerivedData> for NodeIncrementalDeriveSystem {
// //   fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = u64> + '_ {
// //     todo!()
// //   }

// //   fn access(&self, skip_cache: bool) -> impl Fn(&u64) -> Option<SceneNodeDerivedData> + '_ {
// //     todo!()
// //   }
// // }
