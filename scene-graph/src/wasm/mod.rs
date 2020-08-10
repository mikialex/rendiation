use crate::SceneShadingData;
use crate::{Scene, SceneNodeData};
use arena::{AnyHandle, Handle};
use rendiation_math::Mat4;
use rendiation_ral::*;
use rendiation_webgl::WebGLRenderer;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WASMScene {
  // we will use feature gate to control backend selection later
  scene: Scene<WebGLRenderer>,
  handle_pool: Vec<AnyHandle>,
  handle_pool_empty: Vec<usize>,
}

#[wasm_bindgen]
impl WASMScene {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self {
      scene: Scene::new(),
      handle_pool: Vec::new(),
      handle_pool_empty: Vec::new(),
    }
  }

  fn save_handle<T>(&mut self, h: Handle<T>) -> usize {
    if let Some(hole) = self.handle_pool_empty.pop() {
      self.handle_pool[hole] = h.into();
      hole
    } else {
      self.handle_pool.push(h.into());
      self.handle_pool.len() - 1
    }
  }
  fn get_handle(&self, h: usize) -> AnyHandle {
    self.handle_pool[h]
  }
  fn free_handle(&mut self, h: usize) {
    self.handle_pool_empty.push(h);
  }

  #[wasm_bindgen]
  pub fn scene_node_local_matrix_ptr(&mut self, handle: usize) -> *const Mat4<f32> {
    self
      .scene
      .get_node(self.get_handle(handle).into())
      .data()
      .local_matrix
      .as_ptr()
  }

  #[wasm_bindgen]
  pub fn node_add_child_by_handle(&mut self, parent_handle: usize, child_handle: usize) {
    self.scene.node_add_child_by_handle(
      self.get_handle(parent_handle).into(),
      self.get_handle(child_handle).into(),
    );
  }

  #[wasm_bindgen]
  pub fn node_remove_child_by_handle(&mut self, parent_handle: usize, child_handle: usize) {
    self.scene.node_remove_child_by_handle(
      self.get_handle(parent_handle).into(),
      self.get_handle(child_handle).into(),
    );
  }

  #[wasm_bindgen]
  pub fn create_new_node(&mut self) -> usize {
    let h = self.scene.nodes.create_node(SceneNodeData::new());
    self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn free_node(&mut self, h: usize) {
    let hd = self.get_handle(h).into();
    self.scene.free_node(hd);
    self.free_handle(h)
  }

  #[wasm_bindgen]
  pub fn create_render_object(&mut self, geometry_index: usize, shading_index: usize) -> usize {
    let h = self.scene.create_render_object(
      self.get_handle(geometry_index).into(),
      self.get_handle(shading_index).into(),
    );
    self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_render_object(&mut self, h: usize) {
    self.scene.delete_render_object(self.get_handle(h).into());
  }

  #[wasm_bindgen]
  pub fn add_shading(
    &mut self,
    resource: &SceneShadingDescriptor,
    renderer: &mut WebGLRenderer,
  ) -> usize {
    let gpu_shading = WebGLRenderer::create_shading(renderer, resource);
    let h = self
      .scene
      .resources
      .add_shading(SceneShadingData::new(gpu_shading))
      .index();
    self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_shading(&mut self, h: usize) {
    self
      .scene
      .resources
      .shadings
      .remove(self.get_handle(h).into());
  }

  #[wasm_bindgen]
  pub fn add_index_buffer(&mut self, data: &[u32], renderer: &mut WebGLRenderer) -> usize {
    let index_buffer =
      WebGLRenderer::create_index_buffer(renderer, unsafe { std::mem::transmute(data) });
    let h = self.scene.resources.add_index_buffer(index_buffer).index();
    self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_index_buffer(&mut self, h: usize) {
    self
      .scene
      .resources
      .delete_index_buffer(self.get_handle(h).into())
  }
}
