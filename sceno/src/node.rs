use crate::{LightHandle, ModelHandle, Scene, SceneBackend, SceneNodeHandle};
use rendiation_algebra::*;

pub struct SceneNode<T: SceneBackend> {
  pub visible: bool,
  pub local_matrix: Mat4<f32>,
  pub payloads: Vec<SceneNodePayload<T>>,
  pub net_visible: bool,
  pub world_matrix: Mat4<f32>,
}

impl<T: SceneBackend> Default for SceneNode<T> {
  fn default() -> Self {
    Self {
      visible: true,
      local_matrix: Mat4::one(),
      payloads: Vec::new(),
      net_visible: true,
      world_matrix: Mat4::one(),
    }
  }
}

impl<T: SceneBackend> SceneNode<T> {
  pub fn update(&mut self, parent: Option<&Self>) {
    if let Some(parent) = parent {
      self.net_visible = self.visible && parent.net_visible;
      if self.net_visible {
        self.world_matrix = parent.world_matrix * self.local_matrix;
        self.world_matrix = self.world_matrix;
      }
    } else {
      self.world_matrix = self.local_matrix;
      self.net_visible = self.visible
    }
  }

  pub fn set_position(&mut self, position: (f32, f32, f32)) -> &mut Self {
    self.local_matrix = Mat4::translate(position.0, position.1, position.2); // todo
    self
  }

  pub fn with_light(&mut self, light: LightHandle<T>) -> &mut Self {
    self.payloads.push(SceneNodePayload::Light(light));
    self
  }
}

pub enum SceneNodePayload<T: SceneBackend> {
  Model(ModelHandle<T>),
  Light(LightHandle<T>),
  // Camera(Box<dyn Projection>),
}

impl<T: SceneBackend> Scene<T> {
  pub fn get_root_handle(&self) -> SceneNodeHandle<T> {
    self.nodes.get_node(self.nodes.root()).handle()
  }
  pub fn get_root(&self) -> &SceneNode<T> {
    self.nodes.get_node(self.nodes.root()).data()
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode<T> {
    self.get_node_mut(self.nodes.root())
  }

  pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle<T>) {
    self.node_add_child_by_handle(self.nodes.root(), child_handle);
  }

  pub fn node_add_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle<T>,
    child_handle: SceneNodeHandle<T>,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.add(child);
  }

  pub fn node_remove_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle<T>,
    child_handle: SceneNodeHandle<T>,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.remove(child);
  }

  pub fn get_node(&self, handle: SceneNodeHandle<T>) -> &SceneNode<T> {
    self.nodes.get_node(handle).data()
  }

  pub fn get_node_mut(&mut self, handle: SceneNodeHandle<T>) -> &mut SceneNode<T> {
    self.nodes.get_node_mut(handle).data_mut()
  }

  pub fn create_new_node(&mut self) -> &mut SceneNode<T> {
    let node = SceneNode::default();
    let handle = self.nodes.create_node(node);
    self.nodes.get_node_mut(handle).data_mut()
  }

  pub fn free_node(&mut self, handle: SceneNodeHandle<T>) {
    self.nodes.free_node(handle);
  }
}
