// use futures::StreamExt;
// use tree::*;

// use crate::*;

// type ReactiveParentTree =
//   TreeHierarchyDerivedSystem<SceneNodeDerivedData,
// ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>>;

// pub struct NodeIncrementalDeriveSystem {
//   inner: ReactiveParentTree,
// }

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

// pub struct TreeDeriveOutput {
//   inner: ReactiveParentTree,
//   forked_change:
//     Box<dyn Stream<Item = Vec<(usize, Option<DeltaOf<SceneNodeDerivedData>>)>> + Unpin>,
//   scene_id: u64,
// }

// impl VirtualCollection<(u64, usize), Mat4<f32>> for TreeDeriveOutput {
//   fn iter_key(&self, skip_cache: bool) -> impl Iterator<Item = (u64, usize)> + '_ {
//     [].into_iter()
//   }

//   fn access(&self, skip_cache: bool) -> impl Fn(&(u64, usize)) -> Option<Mat4<f32>> + '_ {
//     |_| None
//   }
// }

// impl ReactiveCollection<(u64, usize), Mat4<f32>> for TreeDeriveOutput {
//   type Changes = impl Iterator<Item = CollectionDelta<(u64, usize), Mat4<f32>>> + Clone;

//   fn poll_changes(
//     &mut self,
//     cx: &mut std::task::Context<'_>,
//   ) -> std::task::Poll<Option<Self::Changes>> {
//     let changes = self.forked_change.poll_next_unpin(cx);
//     let s_id = self.scene_id;
//     changes.map(|v| {
//       v.map(|v| {
//         v.iter()
//           .filter_map(|(i, d)| match d {
//             Some(d) => match d {
//               SceneNodeDerivedDataDelta::world_matrix(mat) => {
//                 Some(CollectionDelta::Delta((s_id, *i), *mat))
//               }
//               _ => None,
//             },
//             None => Some(CollectionDelta::Remove((s_id, *i))),
//           })
//           .collect::<Vec<_>>()
//           .into_iter()
//       })
//     })
//   }
// }
