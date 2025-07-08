use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct DrawIndexedIndirectArgsStorage {
  /// The number of vertices to draw.
  pub vertex_count: u32,
  /// The number of instances to draw.
  pub instance_count: u32,
  /// The base index within the index buffer.
  pub base_index: u32,
  /// The value added to the vertex index before indexing into the vertex buffer.
  pub vertex_offset: i32,
  /// The instance ID of the first instance to draw.
  /// Has to be 0, unless INDIRECT_FIRST_INSTANCE is enabled.
  pub base_instance: u32,
}

impl DrawIndexedIndirectArgsStorage {
  pub fn new(
    vertex_count: u32,
    instance_count: u32,
    base_index: u32,
    vertex_offset: i32,
    base_instance: u32,
  ) -> Self {
    Self {
      vertex_count,
      instance_count,
      base_index,
      vertex_offset,
      base_instance,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct DrawIndirectArgsStorage {
  /// The number of vertices to draw.
  pub vertex_count: u32,
  /// The number of instances to draw.
  pub instance_count: u32,
  /// The Index of the first vertex to draw.
  pub base_vertex: u32,
  /// The instance ID of the first instance to draw.
  ///
  /// Has to be 0, INDIRECT_FIRST_INSTANCE is enabled.
  pub base_instance: u32,
}

impl DrawIndirectArgsStorage {
  pub fn new(vertex_count: u32, instance_count: u32, base_vertex: u32, base_instance: u32) -> Self {
    Self {
      vertex_count,
      instance_count,
      base_vertex,
      base_instance,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, Debug, Default, ShaderStruct)]
pub struct DispatchIndirectArgsStorage {
  /// The number of work groups in X dimension.
  pub x: u32,
  /// The number of work groups in Y dimension.
  pub y: u32,
  /// The number of work groups in Z dimension.
  pub z: u32,
}

#[derive(Clone)]
pub enum StorageDrawCommands {
  Indexed(StorageBufferReadonlyDataView<[DrawIndexedIndirectArgsStorage]>),
  NoneIndexed(StorageBufferReadonlyDataView<[DrawIndirectArgsStorage]>),
}

impl StorageDrawCommands {
  pub fn is_index(&self) -> bool {
    match self {
      Self::Indexed(_) => true,
      Self::NoneIndexed(_) => false,
    }
  }

  pub fn cmd_count(&self) -> u32 {
    match self {
      Self::Indexed(v) => v.item_count(),
      Self::NoneIndexed(v) => v.item_count(),
    }
  }

  pub fn indirect_buffer(&self) -> &GPUBufferResourceView {
    match self {
      Self::Indexed(buffer) => &buffer.gpu,
      Self::NoneIndexed(buffer) => &buffer.gpu,
    }
  }

  pub fn bind(&self, builder: &mut BindingBuilder) {
    match self {
      Self::Indexed(v) => builder.bind(v),
      Self::NoneIndexed(v) => builder.bind(v),
    };
  }

  pub fn build(&self, cx: &mut ShaderBindGroupBuilder) -> StorageDrawCommandsInvocation {
    match self {
      Self::Indexed(v) => StorageDrawCommandsInvocation::Indexed(cx.bind_by(v)),
      Self::NoneIndexed(v) => StorageDrawCommandsInvocation::NoneIndexed(cx.bind_by(v)),
    }
  }
}

pub enum StorageDrawCommandsInvocation {
  Indexed(ShaderReadonlyPtrOf<[DrawIndexedIndirectArgsStorage]>),
  NoneIndexed(ShaderReadonlyPtrOf<[DrawIndirectArgsStorage]>),
}

impl StorageDrawCommandsInvocation {
  pub fn array_length(&self) -> Node<u32> {
    match self {
      Self::Indexed(v) => v.array_length(),
      Self::NoneIndexed(v) => v.array_length(),
    }
  }
  pub fn vertex_count(&self, idx: Node<u32>) -> Node<u32> {
    match self {
      Self::Indexed(v) => v.index(idx).vertex_count().load(),
      Self::NoneIndexed(v) => v.index(idx).vertex_count().load(),
    }
  }
}
