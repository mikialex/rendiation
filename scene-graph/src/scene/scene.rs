use super::node::SceneNode;
use crate::{scene_trait::SceneBackend, RALBackend, RenderObject, SceneNodeHandle};
use arena::{Arena, Handle};
use arena_tree::*;

pub type RenderObjectHandle<T> = Handle<RenderObject<T>>;

pub struct Scene<T: RALBackend, S: SceneBackend<T>> {
  pub render_objects: Arena<RenderObject<T>>,
  pub(crate) nodes: ArenaTree<S::NodeData>,
  scene_data: S::SceneData,
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

// pub type CameraHandle = Handle<Box<dyn Camera>>;

// pub struct CameraData {
//   active_camera_index: CameraHandle,
//   cameras: Arena<Box<dyn Camera>>,
// }

// impl CameraData {
//   pub fn set_new_active_camera(&mut self, camera: impl Camera + 'static) -> CameraHandle {
//     let boxed = Box::new(camera);
//     let index = self.cameras.insert(boxed);
//     self.active_camera_index = index;
//     index
//   }

//   pub fn get_active_camera_mut_any(&mut self) -> &mut Box<dyn Camera> {
//     self.cameras.get_mut(self.active_camera_index).unwrap()
//   }

//   pub fn get_active_camera_mut<U: 'static>(&mut self) -> &mut U {
//     self
//       .cameras
//       .get_mut(self.active_camera_index)
//       .unwrap()
//       .as_any_mut()
//       .downcast_mut::<U>()
//       .unwrap()
//   }

//   pub fn get_active_camera<U: 'static>(&mut self) -> &U {
//     self
//       .cameras
//       .get(self.active_camera_index)
//       .unwrap()
//       .as_any()
//       .downcast_ref::<U>()
//       .unwrap()
//   }
// }
