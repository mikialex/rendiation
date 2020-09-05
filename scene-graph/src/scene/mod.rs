use arena::Handle;

pub mod background;
// pub mod culling;
pub mod default_impl;
pub mod node;
pub mod render_engine;
pub mod render_unit;
pub mod scene;

pub use background::*;
// pub use culling::*;
pub use node::*;
pub use render_engine::*;
pub use render_unit::*;
pub use scene::*;

pub type RenderObjectHandle<T> = Handle<RenderObject<T>>;

use super::node::SceneNode;
use crate::{default_impl::DefaultSceneBackend, RALBackend, RenderObject};
use arena::*;
use arena_tree::*;
use rendiation_ral::ResourceManager;

pub trait SceneBackend<T: RALBackend> {
  /// What data stored in tree node
  type NodeData: SceneNodeDataTrait<T>;
  /// Customized info stored directly on scene
  type SceneData: Default;
}

pub trait SceneNodeDataTrait<T: RALBackend>: Default {
  fn update_by_parent(&mut self, parent: Option<&Self>, resource: &mut ResourceManager<T>) -> bool;
  fn provide_render_object<U: Iterator<Item = RenderObject<T>>>(&self) -> U;
}

pub struct Scene<T: RALBackend, S: SceneBackend<T> = DefaultSceneBackend> {
  pub render_objects: Arena<RenderObject<T>>,
  pub(crate) nodes: ArenaTree<S::NodeData>,
  pub scene_data: S::SceneData,
}

impl<T: RALBackend, S: SceneBackend<T>> Scene<T, S> {
  pub fn new() -> Self {
    Self {
      render_objects: Arena::new(),
      nodes: ArenaTree::new(S::NodeData::default()),
      scene_data: S::SceneData::default(),
    }
  }

  pub fn get_root(&self) -> &SceneNode<T, S> {
    self.nodes.get_node(self.nodes.root())
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode<T, S> {
    self.get_node_mut(self.nodes.root())
  }

  pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle<T, S>) {
    self.node_add_child_by_handle(self.nodes.root(), child_handle);
  }
}
