use crate::{default_impl::DefaultSceneBackend, Scene, SceneBackend, RAL};
use arena::Handle;
use arena_tree::*;
use rendiation_ral::ResourceManager;

pub type SceneNodeHandle<T, S = DefaultSceneBackend> = Handle<SceneNode<T, S>>;
pub type SceneNode<T, S = DefaultSceneBackend> = ArenaTreeNode<<S as SceneBackend<T>>::NodeData>;

impl<T: RAL, S: SceneBackend<T>> Scene<T, S> {
  pub fn get_root(&self) -> &SceneNode<T, S> {
    self.nodes.get_node(self.nodes.root())
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode<T, S> {
    self.get_node_mut(self.nodes.root())
  }

  pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle<T, S>) {
    self.node_add_child_by_handle(self.nodes.root(), child_handle);
  }

  pub fn node_add_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle<T, S>,
    child_handle: SceneNodeHandle<T, S>,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.add(child);
  }

  pub fn node_remove_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle<T, S>,
    child_handle: SceneNodeHandle<T, S>,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.remove(child);
  }

  pub fn get_node(&self, handle: SceneNodeHandle<T, S>) -> &SceneNode<T, S> {
    self.nodes.get_node(handle)
  }

  pub fn get_node_mut(&mut self, handle: SceneNodeHandle<T, S>) -> &mut SceneNode<T, S> {
    self.nodes.get_node_mut(handle)
  }

  pub fn create_new_node(&mut self, resource: &mut ResourceManager<T>) -> &mut SceneNode<T, S> {
    let handle = self.nodes.create_node(S::create_node_data(resource));
    self.nodes.get_node_mut(handle)
  }

  pub fn free_node(&mut self, handle: SceneNodeHandle<T, S>) {
    self.nodes.free_node(handle);
  }
}
