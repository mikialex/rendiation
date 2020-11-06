use arena::Handle;

pub mod background;
// pub mod culling;
pub mod default_impl;
pub mod node;
pub mod render_unit;

pub use background::*;
// pub use culling::*;
pub use node::*;
pub use render_unit::*;
use rendiation_render_entity::Camera;

pub type DrawcallHandle<T> = Handle<Drawcall<T>>;

use super::node::SceneNode;
use crate::{default_impl::DefaultSceneBackend, Drawcall, RAL};
use arena::*;
use arena_tree::*;
use rendiation_ral::ResourceManager;

pub trait SceneBackend<T: RAL>: Sized {
  /// What data stored in tree node
  type NodeData: SceneNodeDataTrait<T>;
  /// Customized info stored directly on scene.
  /// Implementor could put extra effect struct, like background on it
  /// and take care of the rendering and updating.
  type SceneData: Default;

  fn create_node_data(resource: &mut ResourceManager<T>) -> Self::NodeData;
}

pub fn render_list<T: RAL, S: SceneBackend<T>>(
  raw_list: &Vec<SceneDrawcall<T, S>>,
  pass: &mut T::RenderPass,
  scene: &Scene<T, S>,
  resources: &ResourceManager<T>,
) {
  raw_list
    .iter()
    .for_each(|d| T::render_drawcall(scene.drawcalls.get(d.drawcall).unwrap(), pass, resources))
}

pub trait SceneNodeDataTrait<T: RAL> {
  type DrawcallIntoIterType;
  fn update(
    &mut self,
    parent: Option<&Self>,
    camera: &Camera,
    resource: &mut ResourceManager<T>,
  ) -> bool;
  fn provide_drawcall<'a>(&self) -> &Self::DrawcallIntoIterType;
}

pub struct SceneNodeDataDrawcallsProvider<'a, P>(pub &'a P);

pub struct Scene<T: RAL, S: SceneBackend<T> = DefaultSceneBackend> {
  pub drawcalls: Arena<Drawcall<T>>,
  pub nodes: ArenaTree<S::NodeData>,
  pub scene_data: S::SceneData,
  reused_traverse_stack: Vec<SceneNodeHandle<T, S>>,
}

impl<T: RAL, S: SceneBackend<T>> Scene<T, S> {
  pub fn new(resource: &mut ResourceManager<T>) -> Self {
    Self {
      drawcalls: Arena::new(),
      nodes: ArenaTree::new(S::create_node_data(resource)),
      scene_data: S::SceneData::default(),
      reused_traverse_stack: Vec::new(),
    }
  }

  pub fn update<'b>(
    &mut self,
    resources: &mut ResourceManager<T>,
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
