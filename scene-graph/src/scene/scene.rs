use super::{background::Background, node::SceneNode, resource::ResourceManager};
use crate::{
  GeometryHandle, RenderData, RenderObject, RenderObjectHandle, SceneGraphBackend, SceneNodeData,
  SceneNodeHandle, ShadingHandle, UniformHandle,
};
use arena::{Arena, Handle};
use arena_tree::*;
use rendiation_render_entity::{Camera, PerspectiveCamera};

pub struct Scene<T: SceneGraphBackend> {
  pub background: Option<Box<dyn Background<T>>>,
  pub cameras: CameraData,
  pub render_objects: Arena<RenderObject<T>>,

  root: SceneNodeHandle<T>,
  pub(crate) nodes: ArenaTree<SceneNodeData<T>>,

  pub resources: ResourceManager<T>,
  pub resource_update_ctx: ResourceUpdateCtx<T>,
}

impl<T: SceneGraphBackend> Scene<T> {
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
      root: Handle::from_raw_parts(0, 0),
      nodes: ArenaTree::new(SceneNodeData::new()),
      resources: ResourceManager::new(),
      resource_update_ctx: ResourceUpdateCtx::new(),
    }
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

  pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle<T>) {
    self.node_add_child_by_handle(self.root, child_handle);
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode<T> {
    self.get_node_mut(self.root)
  }

  pub fn get_node(&self, handle: SceneNodeHandle<T>) -> &SceneNode<T> {
    self.nodes.get_node(handle)
  }

  pub fn get_root(&self) -> &SceneNode<T> {
    self.nodes.get_node(self.root)
  }

  pub fn get_node_mut(&mut self, handle: SceneNodeHandle<T>) -> &mut SceneNode<T> {
    self.nodes.get_node_mut(handle)
  }

  pub fn create_new_node(&mut self) -> &mut SceneNode<T> {
    let handle = self.nodes.create_node(SceneNodeData::new());
    self.nodes.get_node_mut(handle)
  }

  pub fn get_node_render_data(&self, handle: SceneNodeHandle<T>) -> &RenderData {
    &self.nodes.get_node(handle).data().render_data
  }

  pub fn free_node(&mut self, handle: SceneNodeHandle<T>) {
    self.nodes.free_node(handle);
  }

  pub fn create_render_object(
    &mut self,
    geometry_index: GeometryHandle<T>,
    shading_index: ShadingHandle<T>,
  ) -> RenderObjectHandle<T> {
    let obj = RenderObject {
      render_order: 0,
      shading_index,
      geometry_index,
    };
    self.render_objects.insert(obj)
  }

  pub fn delete_render_object(&mut self, index: RenderObjectHandle<T>) {
    self.render_objects.remove(index);
  }
}

pub struct ResourceUpdateCtx<T: SceneGraphBackend> {
  changed_uniforms: Vec<UniformHandle<T>>,
}

impl<T: SceneGraphBackend> ResourceUpdateCtx<T> {
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
