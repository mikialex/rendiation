use std::marker::PhantomData;

use crate::{Background, Material, SceneNode, ShaderComponent};
use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNode};

pub struct SceneEffects {}

pub trait SceneMesh {}

pub type MaterialHandle = Handle<Material>;
pub type MeshHandle = Handle<Box<dyn SceneMesh>>;
pub type ComponentHandle = Handle<Box<dyn ShaderComponent>>;
pub type SceneNodeHandle = ArenaTreeNode<SceneNode>;

pub trait RendererBackend {}

pub struct Scene<R: RendererBackend> {
  phantom: PhantomData<R>,
  nodes: ArenaTree<SceneNode>,
  background: Box<dyn Background>,

  global_effects: SceneEffects,

  meshes: Arena<Box<dyn SceneMesh>>,
  materials: Arena<Material>,
  components: Arena<Box<dyn ShaderComponent>>,
  // samplers: Arena<Sampler>,
  // textures: Arena<Texture>,
  // buffers: Arena<Buffer>,
}

impl<R: RendererBackend> Scene<R> {
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
