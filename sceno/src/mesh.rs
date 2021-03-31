use std::collections::HashMap;

pub trait SceneMesh {}

pub trait IndexBufferSource {}

pub struct IndexBuffer {
  data: Box<dyn IndexBufferSource>,
}

pub trait VertexBufferSource {}

pub struct VertexBuffer {
  data: Box<dyn VertexBufferSource>,
}

pub struct Mesh {
  index_buffer: Option<IndexBuffer>,
  vertex_buffers: HashMap<String, VertexBuffer>,
}
