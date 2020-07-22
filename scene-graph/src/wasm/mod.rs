use crate::{ResourceManager, Scene, SceneNodeData, WebGLBackend, SceneShadingDescriptor, WebGLRenderer};
use arena::{AnyHandle, Handle};
use wasm_bindgen::prelude::*;
use crate::{SceneShadingData, CALBackend};

#[wasm_bindgen]
pub struct WASMScene {
  // we will use feature gate to control backend selection later
  scene: Scene<WebGLBackend>,
}

#[wasm_bindgen]
impl WASMScene {
  #[wasm_bindgen(constructor)]
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
  pub fn node_remove_child_by_handle(&mut self, parent_handle: AnyHandle, child_handle: AnyHandle) {
    self
      .scene
      .node_remove_child_by_handle(parent_handle.into(), child_handle.into());
  }

  #[wasm_bindgen]
  pub fn create_new_node(&mut self) -> AnyHandle {
    self.scene.nodes.create_node(SceneNodeData::new()).into()
  }

  #[wasm_bindgen]
  pub fn free_node(&mut self, h: AnyHandle) {
    self.scene.free_node(h.into());
  }

  #[wasm_bindgen]
  pub fn create_render_object(
    &mut self,
    geometry_index: AnyHandle,
    shading_index: AnyHandle,
  ) -> AnyHandle {
    self
      .scene
      .create_render_object(geometry_index.into(), shading_index.into())
      .into()
  }

  #[wasm_bindgen]
  pub fn delete_render_object(&mut self, h: AnyHandle) {
    self.scene.delete_render_object(h.into());
  }
}

#[wasm_bindgen]
pub struct WASMResourceManager {
  manager: ResourceManager<WebGLBackend>,
}

#[wasm_bindgen]
impl WASMResourceManager {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self {
      manager: ResourceManager::new(),
    }
  }

  #[wasm_bindgen]
  pub fn add_shading(
    &mut self,
    resource: &SceneShadingDescriptor,
    renderer: &mut WebGLRenderer,
  ) -> AnyHandle {
    let gpu_shading = WebGLBackend::create_shading(renderer, resource);
    self.manager.add_shading(SceneShadingData::new(gpu_shading)).index().into()
  }

  #[wasm_bindgen]
  pub fn delete_shading(&mut self, h: AnyHandle) {
    self.manager.shadings.remove(h.into());
  }
}
