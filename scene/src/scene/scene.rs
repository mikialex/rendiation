use crate::{Background, SceneMaterial, SceneNode};
use arena::Arena;
use arena_tree::ArenaTree;

pub struct SceneEffects {}

pub trait SceneMesh {}

pub struct Scene {
  nodes: ArenaTree<SceneNode>,
  background: Box<dyn Background>,

  global_effects: SceneEffects,

  meshes: Arena<Box<dyn SceneMesh>>,
  materials: Arena<Box<dyn SceneMaterial>>,
  // samplers: Arena<Sampler>,
  // textures: Arena<Texture>,
  // buffers: Arena<Buffer>,
}

impl Scene {
  pub fn new() -> Self {
    todo!()
  }

  // pub fn update<'b>(
  //   &mut self,
  //   camera: &Camera,
  //   list: &'b mut SceneDrawcallList<T, S>,
  // ) -> &'b mut SceneDrawcallList<T, S>
  // {
  //   let root = self.get_root().handle();
  //   list.inner.clear();
  //   self.nodes.traverse(
  //     root,
  //     &mut self.reused_traverse_stack,
  //     |this: &mut SceneNode<T, S>, parent: Option<&mut SceneNode<T, S>>| {
  //       let this_handle = this.handle();
  //       let node_data = this.data_mut();

  //       let net_visible = node_data.update(parent.map(|p| p.data()), camera, resources);

  //       if net_visible {
  //         list
  //           .inner
  //           .extend(
  //             node_data
  //               .provide_drawcall()
  //               .into_iter()
  //               .map(|&drawcall| SceneDrawcall {
  //                 drawcall,
  //                 node: this_handle,
  //               }),
  //           );
  //         NextTraverseVisit::VisitChildren
  //       } else {
  //         NextTraverseVisit::SkipChildren
  //       }
  //     },
  //   );
  //   list
  // }
}
