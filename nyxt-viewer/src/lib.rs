use std::{cell::RefCell, rc::Rc};

use arena::{AnyHandle, Handle};
use rendiation_math::Mat4;
use rendiation_ral::*;

use rendiation_scenegraph::default_impl::*;
use rendiation_scenegraph::*;

use rendiation_webgl::{WebGL, WebGLRenderer};
use wasm_bindgen::prelude::*;

mod geometry;
use geometry::*;

pub type GFX = WebGL;

// #[wasm_bindgen]
// #[derive(Debug, Copy, Clone)]
// pub struct Fog {
//   pub data: f32,
// }

// #[wasm_bindgen]
// pub struct FogWASM {
//   index: usize,
//   resource: Weak<RefCell<ResourceManager<GFX>>>,
// }

// #[wasm_bindgen]
// pub struct BlockShadingParamGroupWASM {
//   index: usize,
//   resource: Weak<RefCell<ResourceManager<GFX>>>,
// }

// // create WASMWrappedItem_Fog from WASMScene by default value

// #[wasm_bindgen]
// impl BlockShadingParamGroupWASM {
//   #[wasm_bindgen(setter)]
//   pub fn set_fog(&mut self, fog: &FogWASM) {}
// }

#[wasm_bindgen]
pub struct WASMScene {
  resource: Rc<RefCell<ResourceManager<GFX>>>,
  scene: Scene<GFX>,
  handle_pool: Vec<AnyHandle>,
  handle_pool_empty: Vec<usize>,
}

#[wasm_bindgen]
impl WASMScene {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    console_error_panic_hook::set_once();
    Self {
      resource: Rc::new(RefCell::new(ResourceManager::new())),
      scene: Scene::new(),
      handle_pool: Vec::new(),
      handle_pool_empty: Vec::new(),
    }
  }

  fn mutate_resource<T>(&self, mutator: impl FnOnce(&mut ResourceManager<GFX>) -> T) -> T {
    let mut resource = self.resource.borrow_mut();
    mutator(&mut resource)
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
    data: &WASMAttributeBufferU16,
    renderer: &mut WebGLRenderer,
  ) -> usize {
    let index_buffer =
      GFX::create_index_buffer(renderer, bytemuck::cast_slice(data.buffer.as_slice()));
    let h = self.mutate_resource(|r| r.add_index_buffer(index_buffer).index());
    self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_index_buffer(&mut self, h: usize) {
    self.mutate_resource(|r| r.delete_index_buffer(self.get_handle(h).into()));
    self.free_handle(h);
  }

  #[wasm_bindgen]
  pub fn add_vertex_buffer(
    &mut self,
    data: &WASMAttributeBufferF32,
    renderer: &mut WebGLRenderer,
    // WebGLAttributeBufferFormat => RALVertexAttributeFormat
  ) -> usize {
    let vertex_buffer = GFX::create_vertex_buffer(
      renderer,
      bytemuck::cast_slice(data.buffer.as_slice()),
      RALVertexBufferDescriptor {
        byte_stride: 4,
        attributes: vec![RALVertexAttributeBufferDescriptor {
          byte_offset: 0,
          format: RALVertexAttributeFormat::Float,
        }],
      },
    );
    let h = self.mutate_resource(|r| r.add_vertex_buffer(vertex_buffer).index());
    self.save_handle(h)
  }

  #[wasm_bindgen]
  pub fn delete_vertex_buffer(&mut self, h: usize) {
    self.mutate_resource(|r| r.delete_vertex_buffer(self.get_handle(h).into()));
    self.free_handle(h);
  }

  #[wasm_bindgen]
  pub fn add_geometry(&mut self, geometry: &WASMGeometry) -> usize {
    let h = self.mutate_resource(|r| r.add_geometry(geometry.to_geometry_resource_instance()));
    self.save_handle(h)
  }
}

pub enum WebGLAttributeBufferFormat {
  Float,
  Float2,
  Float3,
}
