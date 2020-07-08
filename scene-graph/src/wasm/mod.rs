use crate::{ResourceManager, Scene, WebGLBackend};
use arena::Handle;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WASMScene {
  // we will use feature gate to control backend selection later
  scene: Scene<WebGLBackend>,
}

#[wasm_bindgen]
impl WASMScene {
  #[wasm_bindgen]
  pub fn new() -> Self {
    Self {
      scene: Scene::new(),
    }
  }

  #[wasm_bindgen]
  pub fn node_add_child_by_handle(
    &mut self,
    parent_handle: usize,
    parent_handle_generation: u64,
    child_handle: usize,
    child_handle_generation: u64,
  ) {
    self.scene.node_add_child_by_handle(
      Handle::from_raw_parts(parent_handle, parent_handle_generation),
      Handle::from_raw_parts(child_handle, child_handle_generation),
    );
  }

  #[wasm_bindgen]
  pub fn node_remove_child_by_handle(
    &mut self,
    parent_handle: usize,
    parent_handle_generation: u64,
    child_handle: usize,
    child_handle_generation: u64,
  ) {
    self.scene.node_remove_child_by_handle(
      Handle::from_raw_parts(parent_handle, parent_handle_generation),
      Handle::from_raw_parts(child_handle, child_handle_generation),
    );
  }

  // pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle<T>) {
  //   self.node_add_child_by_handle(self.root, child_handle);
  // }

  // pub fn get_root_node_mut(&mut self) -> &mut SceneNode<T> {
  //   self.get_node_mut(self.root)
  // }

  // pub fn get_node(&self, handle: SceneNodeHandle<T>) -> &SceneNode<T> {
  //   self.nodes.get_node(handle)
  // }

  // pub fn get_root(&self) -> &SceneNode<T> {
  //   self.nodes.get_node(self.root)
  // }

  // pub fn get_node_mut(&mut self, handle: SceneNodeHandle<T>) -> &mut SceneNode<T> {
  //   self.nodes.get_node_mut(handle)
  // }

  #[wasm_bindgen]
  pub fn create_new_node(&mut self) -> usize {
    let handle = self.scene.create_new_node();
    todo!()
    // self.nodes.get_node_mut(handle)
  }
  #[wasm_bindgen]
  pub fn get_node_g(&mut self) -> u64 {
    todo!()
    // let handle = self.nodes.create_node(SceneNodeData::new());
    // self.nodes.get_node_mut(handle)
  }

  // pub fn get_node_render_data(&self, handle: SceneNodeHandle<T>) -> &RenderData {
  //   &self.nodes.get_node(handle).data().render_data
  // }

  #[wasm_bindgen]
  pub fn free_node(&mut self, h: usize, g: u64) {
    self.scene.free_node(Handle::from_raw_parts(h, g));
  }

  // pub fn create_render_object(
  //   &mut self,
  //   geometry_index: GeometryHandle<T>,
  //   shading_index: ShadingHandle<T>,
  // ) -> RenderObjectHandle<T> {
  //   let obj = RenderObject {
  //     render_order: 0,
  //     shading_index,
  //     geometry_index,
  //   };
  //   self.render_objects.insert(obj)
  // }

  // pub fn delete_render_object(&mut self, index: RenderObjectHandle<T>) {
  //   self.render_objects.remove(index);
  // }
}

#[wasm_bindgen]
pub struct WASMResourceManager {
  manager: ResourceManager<WebGLBackend>,
}

#[wasm_bindgen]
impl WASMResourceManager {
  #[wasm_bindgen]
  pub fn new() -> Self {
    Self {
      manager: ResourceManager::new(),
    }
  }
}
