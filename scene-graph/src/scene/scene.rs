use super::{background::Background, node::SceneNode, resource::ResourceManager};
use crate::{RALBackend, RenderObject, SceneNodeData, SceneNodeHandle, UniformHandle};
use arena::{Arena, Handle};
use arena_tree::*;
use rendiation_render_entity::{Camera, PerspectiveCamera};

pub struct Scene<T: RALBackend> {
  pub background: Option<Box<dyn Background<T>>>,
  pub cameras: CameraData,
  pub render_objects: Arena<RenderObject<T>>,

  pub(crate) nodes: ArenaTree<SceneNodeData<T>>,

  pub resources: ResourceManager<T>,
  pub resource_update_ctx: ResourceUpdateCtx<T>,
}

impl<T: RALBackend> Scene<T> {
  pub fn new() -> Self {
    let camera_default = Box::new(PerspectiveCamera::new());

    let mut cameras: Arena<Box<dyn Camera>> = Arena::new();
    let active_camera_index = cameras.insert(camera_default);

    Self {
      background: None,
      cameras: CameraData {
        active_camera_index,
        cameras,
      },
      render_objects: Arena::new(),
      nodes: ArenaTree::new(SceneNodeData::new()),
      resources: ResourceManager::new(),
      resource_update_ctx: ResourceUpdateCtx::new(),
    }
  }

  pub fn get_root(&self) -> &SceneNode<T> {
    self.nodes.get_node(self.nodes.root())
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode<T> {
    self.get_node_mut(self.nodes.root())
  }

  pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle<T>) {
    self.node_add_child_by_handle(self.nodes.root(), child_handle);
  }
}

pub struct ResourceUpdateCtx<T: RALBackend> {
  changed_uniforms: Vec<UniformHandle<T>>,
}

impl<T: RALBackend> ResourceUpdateCtx<T> {
  pub fn new() -> Self {
    Self {
      changed_uniforms: Vec::new(),
    }
  }
  pub fn notify_uniform_update(&mut self, index: UniformHandle<T>) {
    self.changed_uniforms.push(index)
  }
}

pub type CameraHandle = Handle<Box<dyn Camera>>;

pub struct CameraData {
  active_camera_index: CameraHandle,
  cameras: Arena<Box<dyn Camera>>,
}

impl CameraData {
  pub fn set_new_active_camera(&mut self, camera: impl Camera + 'static) -> CameraHandle {
    let boxed = Box::new(camera);
    let index = self.cameras.insert(boxed);
    self.active_camera_index = index;
    index
  }

  pub fn get_active_camera_mut_any(&mut self) -> &mut Box<dyn Camera> {
    self.cameras.get_mut(self.active_camera_index).unwrap()
  }

  pub fn get_active_camera_mut<U: 'static>(&mut self) -> &mut U {
    self
      .cameras
      .get_mut(self.active_camera_index)
      .unwrap()
      .as_any_mut()
      .downcast_mut::<U>()
      .unwrap()
  }

  pub fn get_active_camera<U: 'static>(&mut self) -> &U {
    self
      .cameras
      .get(self.active_camera_index)
      .unwrap()
      .as_any()
      .downcast_ref::<U>()
      .unwrap()
  }
}
