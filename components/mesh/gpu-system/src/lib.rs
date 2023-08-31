#![feature(strict_provenance)]

use rendiation_shader_api::*;
use rendiation_webgpu::*;
use slab::Slab;

mod allocator;
use allocator::*;

mod draw;
pub use draw::*;

// todo, support runtime size by query client limitation
pub const MAX_STORAGE_BINDING_ARRAY_LENGTH: usize = 8192;

pub type MeshSystemMeshHandle = u32;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct DrawMetaData {
  pub start: u32,
  pub count: u32,
  pub vertex_info: DrawVertexIndirectInfo,
}

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

#[repr(C)]
#[derive(Clone, Copy, ShaderStruct, Zeroable, Pod)]
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

pub struct GPUBindlessMeshSystem {
  meta_data: Slab<DrawMetaData>,

  index_buffer: GPUSubAllocateBuffer<u32>,

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

impl GPUBindlessMeshSystem {
  pub fn new(gpu: &GPU) -> Self {
    let info = gpu.info();
    let mut bindless_effectively_supported = info
      .supported_features
      .contains(Features::BUFFER_BINDING_ARRAY)
      && info
        .supported_features
        .contains(Features::PARTIALLY_BOUND_BINDING_ARRAY)
      && info
        .supported_features
        .contains(Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING);

    // we estimate that the buffer used except under the binding system will not exceed 128 per
    // shader stage
    if info.supported_limits.max_sampled_textures_per_shader_stage
      < MAX_STORAGE_BINDING_ARRAY_LENGTH as u32 + 128
      || info.supported_limits.max_samplers_per_shader_stage
        < MAX_STORAGE_BINDING_ARRAY_LENGTH as u32 + 128
    {
      bindless_effectively_supported = false;
    }

    let bindless_enabled = bindless_effectively_supported;

    todo!()
  }

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
}

// impl Stream for GPUBindlessMeshSystem {

// }

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
