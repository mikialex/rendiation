use crate::{Camera, SceneNode};
use arena::Arena;
use arena_tree::{ArenaTree, NextTraverseVisit};

pub struct Scene {
  nodes: ArenaTree<SceneNode>,
  background: Box<dyn Background>,

  meshes: Arena<Mesh>,
  materials: Arena<Material>,

  samplers: Arena<Sampler>,
  textures: Arena<Texture>,
  buffers: Arena<Buffer>,
}

impl Scene {
  pub fn new() -> Self {
    Self {
      drawcalls: Arena::new(),
      nodes: ArenaTree::new(S::create_node_data(resource)),
      scene_data: S::SceneData::default(),
      reused_traverse_stack: Vec::new(),
    }
  }

  pub fn update<'b>(
    &mut self,
    camera: &Camera,
    list: &'b mut SceneDrawcallList<T, S>,
  ) -> &'b mut SceneDrawcallList<T, S>
  where
    for<'a> &'a <S::NodeData as SceneNodeDataTrait<T>>::DrawcallIntoIterType:
      IntoIterator<Item = &'a DrawcallHandle<T>>,
    // maybe we could let SceneNodeDataTrait impl IntoExactSizeIterator for simplicity
  {
    let root = self.get_root().handle();
    list.inner.clear();
    self.nodes.traverse(
      root,
      &mut self.reused_traverse_stack,
      |this: &mut SceneNode<T, S>, parent: Option<&mut SceneNode<T, S>>| {
        let this_handle = this.handle();
        let node_data = this.data_mut();

        let net_visible = node_data.update(parent.map(|p| p.data()), camera, resources);

        if net_visible {
          list
            .inner
            .extend(
              node_data
                .provide_drawcall()
                .into_iter()
                .map(|&drawcall| SceneDrawcall {
                  drawcall,
                  node: this_handle,
                }),
            );
          NextTraverseVisit::VisitChildren
        } else {
          NextTraverseVisit::SkipChildren
        }
      },
    );
    list
  }
}

pub trait SceneRenderer {
  fn render(&mut self, scene: &mut Scene);
}
