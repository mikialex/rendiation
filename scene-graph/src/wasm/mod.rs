use crate::{default_impl::SceneNodeData, Scene};
use arena::{AnyHandle, Handle};
use rendiation_math::Mat4;
use rendiation_mesh_buffer::wasm::{WASMAttributeBufferF32, WASMAttributeBufferU16, WASMGeometry};
use rendiation_ral::*;
use rendiation_render_entity::PerspectiveProjection;
use rendiation_webgl::{WebGL, WebGLRenderer};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WASMScene {
  resource: ResourceManager<WebGL>,
  scene: Scene<WebGL>,
  _camera: PerspectiveProjection,
  handle_pool: Vec<AnyHandle>,
  handle_pool_empty: Vec<usize>,
}

#[wasm_bindgen]
impl WASMScene {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    Self {
      resource: ResourceManager::new(),
      scene: Scene::new(),
      _camera: PerspectiveProjection::default(),
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
  pub fn create_drawcall(&mut self, geometry_index: usize, shading_index: usize) -> usize {
    let h = self
      .scene
      .create_drawcall::<AnyPlaceHolder, AnyGeometryProvider>(
        self.get_handle(geometry_index).into(),
        self.get_handle(shading_index).into(),
      );
    self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_drawcall(&mut self, h: usize) {
    self.scene.delete_drawcall(self.get_handle(h).into());
  }

  #[wasm_bindgen]
  pub fn add_shading(
    &mut self,
    // shading: Box<dyn Any>,
  ) -> usize {
    // let gpu_shading = WebGLRenderer::create_shading(renderer, resource);
    // let h = self
    //   .scene
    //   .resources
    //   .add_shading(SceneShadingData::new(gpu_shading))
    //   .index();
    // self.save_handle(h)
    todo!()
  }

  #[wasm_bindgen]
  pub fn delete_shading(&mut self, _h: usize) {
    // self
    //   .resource
    //   .shadings
    //   .delete_shading(self.get_handle(h).into());
  }

  #[wasm_bindgen]
  pub fn add_index_buffer(
    &mut self,
    _data: &WASMAttributeBufferU16,
    _renderer: &mut WebGLRenderer,
  ) -> usize {
    todo!()
    // let index_buffer = WebGLRenderer::create_index_buffer(renderer, todo!());
    // let h = self.resource.add_index_buffer(index_buffer).index();
    // self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_index_buffer(&mut self, h: usize) {
    self.resource.delete_index_buffer(self.get_handle(h).into());
    // self.delete_handle(h);
  }

  #[wasm_bindgen]
  pub fn add_vertex_buffer(
    &mut self,
    _data: &WASMAttributeBufferF32,
    _renderer: &mut WebGLRenderer,
  ) -> usize {
    todo!()
    // let index_buffer = WebGLRenderer::create_vertex_buffer(renderer, unsafe { todo!() });
    // let h = self.resource.add_index_buffer(index_buffer).index();
    // self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_vertex_buffer(&mut self, h: usize) {
    self.resource.delete_index_buffer(self.get_handle(h).into())
  }

  #[wasm_bindgen]
  pub fn add_geometry(&mut self, geometry: &WASMGeometry) -> usize {
    let h = self
      .resource
      .add_geometry(geometry.to_geometry_resource_instance());
    self.save_handle(h)
  }
}
