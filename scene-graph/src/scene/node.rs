use crate::{default_impl::DefaultSceneBackend, RALBackend, Scene, SceneBackend};
use arena::Handle;
use arena_tree::*;

pub type SceneNodeHandle<T, S = DefaultSceneBackend> = Handle<SceneNode<T, S>>;
pub type SceneNode<T, S = DefaultSceneBackend> = ArenaTreeNode<<S as SceneBackend<T>>::NodeData>;

impl<T: RALBackend, S: SceneBackend<T>> Scene<T, S> {
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

  pub fn create_new_node(&mut self) -> &mut SceneNode<T, S> {
    let handle = self.nodes.create_node(S::NodeData::default());
    self.nodes.get_node_mut(handle)
  }

  pub fn free_node(&mut self, handle: SceneNodeHandle<T, S>) {
    self.nodes.free_node(handle);
  }
}
