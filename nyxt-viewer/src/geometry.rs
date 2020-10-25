use std::{cell::RefCell, rc::Weak};

use rendiation_ral::{
  AnyGeometryProvider, GeometryResourceInstance, IndexBufferHandle,
  RALVertexAttributeBufferDescriptor, RALVertexAttributeFormat, RALVertexBufferDescriptor,
  ResourceManager, VertexBufferHandle, RAL,
};
use wasm_bindgen::prelude::*;

use crate::{NyxtViewer, GFX};

#[wasm_bindgen]
pub struct AttributeBufferF32WASM {
  #[wasm_bindgen(skip)]
  pub buffer: Vec<f32>,
  #[wasm_bindgen(skip)]
  pub stride: usize,
}

#[wasm_bindgen]
pub struct AttributeBufferU16WASM {
  #[wasm_bindgen(skip)]
  pub buffer: Vec<u16>,
  #[wasm_bindgen(skip)]
  pub stride: usize,
}

#[wasm_bindgen]
impl AttributeBufferF32WASM {
  #[wasm_bindgen(constructor)]
  pub fn new(buffer: &[f32], stride: usize) -> Self {
    Self {
      buffer: buffer.to_owned(),
      stride,
    }
  }
}

#[wasm_bindgen]
impl AttributeBufferU16WASM {
  #[wasm_bindgen(constructor)]
  pub fn new(buffer: &[u16], stride: usize) -> Self {
    Self {
      buffer: buffer.to_owned(),
      stride,
    }
  }
}

#[wasm_bindgen]
pub struct IndexBufferWASM {
  handle: IndexBufferHandle<GFX>,
  resource: Weak<RefCell<ResourceManager<GFX>>>,
}

// struct HandleAndResource<Handle, Resource, Item, Mutator> {
//   handle: IndexBufferHandle<GFX>,
//   resource: Weak<RefCell<ResourceManager<GFX>>>,
//   mutator: Mutator
// }

// impl<Handle, Resource, Item, Mutator> HandleAndResource<Handle, Resource, Item, Mutator> {
//   fn
// }

#[wasm_bindgen]
impl IndexBufferWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(viewer: &mut NyxtViewer, buffer: &AttributeBufferU16WASM) -> Self {
    let index_buffer = GFX::create_index_buffer(
      &mut viewer.renderer,
      bytemuck::cast_slice(buffer.buffer.as_slice()),
    );
    let handle = viewer.mutate_resource(|r| r.add_index_buffer(index_buffer).index());
    Self {
      handle,
      resource: viewer.make_resource(),
    }
  }
}

impl Drop for IndexBufferWASM {
  fn drop(&mut self) {
    todo!()
    // let handle = self.mutate_resource(|r| r.delete_index_buffer(index_buffer));
  }
}

#[wasm_bindgen]
pub struct VertexBufferWASM {
  handle: VertexBufferHandle<GFX>,
  resource: Weak<RefCell<ResourceManager<GFX>>>,
}

#[wasm_bindgen]
impl VertexBufferWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(viewer: &mut NyxtViewer, buffer: &AttributeBufferF32WASM) -> Self {
    let vertex_buffer = GFX::create_vertex_buffer(
      &mut viewer.renderer,
      bytemuck::cast_slice(buffer.buffer.as_slice()),
      RALVertexBufferDescriptor {
        byte_stride: 4,
        attributes: vec![RALVertexAttributeBufferDescriptor {
          byte_offset: 0,
          format: RALVertexAttributeFormat::Float,
        }],
      },
    );
    let handle = viewer.mutate_resource(|r| r.add_vertex_buffer(vertex_buffer).index());
    Self {
      handle,
      resource: viewer.make_resource(),
    }
  }
}

impl Drop for VertexBufferWASM {
  fn drop(&mut self) {
    todo!()
  }
}

#[wasm_bindgen]
pub struct WASMGeometry {
  // data: GeometryResourceInstance<WebGLRenderer>,
  pub index: Option<usize>,
  pub position: usize,
  pub normal: Option<usize>,
  pub uv: Option<usize>,
}

impl WASMGeometry {
  pub fn to_geometry_resource_instance(
    &self,
  ) -> GeometryResourceInstance<GFX, AnyGeometryProvider> {
    todo!()
  }
}

// #[wasm_bindgen]
// impl WASMGeometry {
//   #[wasm_bindgen(constructor)]
//   pub fn new(
//     index: Option<IndexBufferWASM>,
//     position: VertexBufferWASM,
//     normal: Option<VertexBufferWASM>,
//     uv: Option<VertexBufferWASM>,
//   ) -> Self {
//     Self {
//       index: index.map(|d| d.handle),
//       position: position.handle,
//       normal: normal.map(|d| d.handle),
//       uv: uv.map(|d| d.handle),
//     }
//   }
// }

impl Drop for WASMGeometry {
  fn drop(&mut self) {
    todo!()
    // let handle = self.mutate_resource(|r| r.delete_index_buffer(index_buffer));
  }
}
