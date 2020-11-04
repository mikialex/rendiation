use rendiation_mesh_buffer::vertex::Vertex;
use rendiation_ral::{
  GeometryResourceInstance, IndexBufferHandle, InputStepMode, VertexAttributeDescriptor,
  VertexBufferDescriptor, VertexBufferHandle, VertexFormat, RAL,
};
use wasm_bindgen::prelude::*;

use crate::{NyxtViewer, NyxtViewerHandle, NyxtViewerHandledObject, NyxtViewerInner, GFX};

pub enum WebGLAttributeBufferFormat {
  Float,
  Float2,
  Float3,
}

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
  #[wasm_bindgen(skip)]
  pub inner: NyxtViewerHandledObject<IndexBufferHandleWrap>,
}

#[derive(Copy, Clone)]
pub struct IndexBufferHandleWrap(IndexBufferHandle<GFX>);

impl NyxtViewerHandle for IndexBufferHandleWrap {
  type Item = <GFX as RAL>::IndexBuffer;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.resource.get_index_buffer(self.0).resource()
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.resource.delete_index_buffer(self.0)
  }
}

#[wasm_bindgen]
impl IndexBufferWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(viewer: &mut NyxtViewer, buffer: &AttributeBufferU16WASM) -> Self {
    let handle = viewer.mutate_inner(|inner| {
      let buffer = GFX::create_index_buffer(
        &mut inner.renderer,
        bytemuck::cast_slice(buffer.buffer.as_slice()),
      );
      inner.resource.add_index_buffer(buffer).index()
    });
    Self {
      inner: viewer.make_handle_object(IndexBufferHandleWrap(handle)),
    }
  }
}

#[wasm_bindgen]
pub struct VertexBufferWASM {
  inner: NyxtViewerHandledObject<VertexBufferHandleWrap>,
}

#[derive(Copy, Clone)]
pub struct VertexBufferHandleWrap(VertexBufferHandle<GFX>);
impl NyxtViewerHandle for VertexBufferHandleWrap {
  type Item = <GFX as RAL>::VertexBuffer;

  fn get(self, inner: &NyxtViewerInner) -> &Self::Item {
    inner.resource.get_vertex_buffer(self.0).resource()
  }
  fn free(self, inner: &mut NyxtViewerInner) {
    inner.resource.delete_vertex_buffer(self.0)
  }
}

#[wasm_bindgen]
impl VertexBufferWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(viewer: &mut NyxtViewer, buffer: &AttributeBufferF32WASM) -> Self {
    let handle = viewer.mutate_inner(|inner| {
      let buffer = GFX::create_vertex_buffer(
        &mut inner.renderer,
        bytemuck::cast_slice(buffer.buffer.as_slice()),
        VertexBufferDescriptor {
          stride: 4,
          step_mode: InputStepMode::Vertex,
          attributes: &[VertexAttributeDescriptor {
            shader_location: 0,
            offset: 0,
            format: VertexFormat::Float,
          }],
        },
      );
      inner.resource.add_vertex_buffer(buffer).index()
    });
    Self {
      inner: viewer.make_handle_object(VertexBufferHandleWrap(handle)),
    }
  }
}

impl Drop for VertexBufferWASM {
  fn drop(&mut self) {
    todo!()
  }
}

#[wasm_bindgen]
pub struct IndexedVertexGeometryWASM {
  #[wasm_bindgen(skip)]
  pub index: IndexBufferHandleWrap,
  #[wasm_bindgen(skip)]
  pub position: VertexBufferHandleWrap,
  #[wasm_bindgen(skip)]
  pub normal: VertexBufferHandleWrap,
  #[wasm_bindgen(skip)]
  pub uv: VertexBufferHandleWrap,
  // geometry_instance: GeometryResourceInstance<GFX, Vertex>,
}

// impl IndexedVertexGeometryWASM {
//   pub fn to_geometry_resource_instance(
//     &self,
//   ) -> GeometryResourceInstance<GFX, AnyGeometryProvider> {
//     todo!()
//   }
// }

#[wasm_bindgen]
impl IndexedVertexGeometryWASM {
  #[wasm_bindgen(constructor)]
  pub fn new(
    index: &IndexBufferWASM, // wasm not support Option<&T>??
    position: &VertexBufferWASM,
    normal: &VertexBufferWASM,
    uv: &VertexBufferWASM,
  ) -> Self {
    let mut _instance = GeometryResourceInstance::<GFX, Vertex>::new();
    Self {
      index: index.inner.handle,
      position: position.inner.handle,
      normal: normal.inner.handle,
      uv: uv.inner.handle,
    }
  }
}

impl Drop for IndexedVertexGeometryWASM {
  fn drop(&mut self) {
    todo!()
  }
}
