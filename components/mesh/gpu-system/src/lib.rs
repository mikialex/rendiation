#![feature(strict_provenance)]

use __core::ops::Range;
use fast_hash_collection::FastHashMap;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use slab::Slab;

mod allocator;
use allocator::*;

// todo, support runtime size by query client limitation
pub const MAX_STORAGE_BINDING_ARRAY_LENGTH: usize = 8192;

pub type MeshSystemMeshHandle = u32;

#[derive(Clone)]
pub struct MeshSystemMeshInstance {
  pub inner: u32,
  // todo
}

impl Drop for MeshSystemMeshInstance {
  fn drop(&mut self) {
    todo!()
  }
}

pub struct MeshGPUDrivenSystem {
  next_id: usize,
  // range to index buffer,index to vertex_indirect_buffer
  draw_ranges: FastHashMap<u32, (Range<u32>, DrawVertexIndirectInfo)>,

  index_buffer: GPUSubAllocateBuffer<u32>,
  // todo, not store here, should generate each time
  vertex_address_buffer: StorageBufferReadOnlyDataView<[DrawVertexIndirectInfo]>,

  position_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec3<f32>]>>,
  normal_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec3<f32>]>>,
  normal_uv_buffers: Slab<StorageBufferReadOnlyDataView<[Vec2<f32>]>>,

  bindless_position_vertex_buffers: BindingResourceArray<
    StorageBufferReadOnlyDataView<[Vec3<f32>]>,
    MAX_STORAGE_BINDING_ARRAY_LENGTH,
  >,
  bindless_normal_vertex_buffers: BindingResourceArray<
    StorageBufferReadOnlyDataView<[Vec3<f32>]>,
    MAX_STORAGE_BINDING_ARRAY_LENGTH,
  >,
  bindless_uv_vertex_buffers: BindingResourceArray<
    StorageBufferReadOnlyDataView<[Vec2<f32>]>,
    MAX_STORAGE_BINDING_ARRAY_LENGTH,
  >,
}

// impl Stream for MeshGPUDrivenSystem {

// }

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct DrawVertexIndirectInfo {
  pub position_buffer_id: u32,
  pub position_buffer_offset: u32,
  pub normal_buffer_id: u32,
  pub normal_buffer_offset: u32,
  pub uv_buffer_id: u32,
  pub uv_buffer_offset: u32,
}

#[derive(Clone, Copy, ShaderStruct)]
pub struct Vertex {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub uv: Vec2<f32>,
}

#[repr(C)]
#[derive(Clone, Copy, ShaderStruct)]
pub struct DrawIndirect {
  /// The number of vertices to draw.
  pub vertex_count: u32,
  /// The number of instances to draw.
  pub instance_count: u32,
  /// The Index of the first vertex to draw.
  pub base_vertex: u32,
  /// The instance ID of the first instance to draw.
  /// Has to be 0, unless INDIRECT_FIRST_INSTANCE is enabled.
  pub base_instance: u32,
}

impl MeshGPUDrivenSystem {
  pub fn maintain(&mut self) {
    // todo check any changed
  }

  pub fn create_mesh_instance(
    &mut self,
    position: Vec<f32>,
    normal: Vec<f32>,
    uv: Vec<f32>,
  ) -> MeshSystemMeshInstance {
    todo!()
  }

  pub fn generate_draw_command_buffer_from_host(
    &self,
    buffer: impl Iterator<Item = MeshSystemMeshHandle> + 'static,
  ) -> impl Iterator<Item = (DrawIndirect, DrawVertexIndirectInfo)> + '_ {
    buffer.enumerate().map(|(i, handle)| {
      let (range, vertex_info) = self.draw_ranges.get(&handle).unwrap();
      let draw_indirect = DrawIndirect {
        vertex_count: range.end - range.start,
        instance_count: 1,
        base_vertex: range.start,
        base_instance: i as u32, // we rely on this to get draw id. https://www.g-truc.net/post-0518.html
      };
      (draw_indirect, *vertex_info)
    })
  }

  // /// user could use this in their compute shader to generate the buffer we want
  // pub fn generate_draw_command_on_device(
  //   &self,
  //   handle: Node<u32>,
  // ) -> (Node<DrawIndirect>, Node<DrawVertexIndirectInfo>) {
  //   todo!()
  // }

  pub fn fetch_vertex_on_device(
    &self,
    vertex: ShaderVertexBuilder,
    mut binding: ShaderBindGroupDirectBuilder,
  ) -> ENode<Vertex> {
    let draw_id = vertex.query::<VertexInstanceIndex>().unwrap();
    let vertex_id = vertex.query::<VertexIndex>().unwrap();

    let vertex_addresses = binding.bind_by(&self.vertex_address_buffer);
    let vertex_address = vertex_addresses.index(draw_id).load().expand();

    let position = binding.bind_by(&self.bindless_position_vertex_buffers);
    let position = position.index(vertex_address.position_buffer_id);
    let position = position
      .index(vertex_address.position_buffer_offset + vertex_id)
      .load();

    let normal = binding.bind_by(&self.bindless_normal_vertex_buffers);
    let normal = normal.index(vertex_address.position_buffer_id);
    let normal = normal
      .index(vertex_address.normal_buffer_offset + vertex_id)
      .load();

    let uv = binding.bind_by(&self.bindless_uv_vertex_buffers);
    let uv = uv.index(vertex_address.position_buffer_id);
    let uv = uv.index(vertex_address.uv_buffer_offset + vertex_id).load();

    ENode::<Vertex> {
      position,
      normal,
      uv,
    }
  }
}
