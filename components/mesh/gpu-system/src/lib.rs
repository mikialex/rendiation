#![feature(strict_provenance)]

use std::sync::{Arc, RwLock, Weak};

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
  pub normal_buffer_id: u32,
  pub uv_buffer_id: u32,
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

#[derive(Clone)]
pub struct GPUBindlessMeshSystem {
  inner: Arc<RwLock<GPUBindlessMeshSystemInner>>,
}

pub struct GPUBindlessMeshSystemInner {
  any_changed: bool,
  meta_data: Slab<DrawMetaData>,

  index_buffer: GPUSubAllocateBuffer<u32>,

  position_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec3<f32>]>>,
  normal_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec3<f32>]>>,
  uv_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec2<f32>]>>,

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
  pub fn new(gpu: &GPU) -> Option<Self> {
    let info = gpu.info();
    let mut bindless_effectively_supported = info
      .supported_features
      .contains(Features::BUFFER_BINDING_ARRAY)
      && info
        .supported_features
        .contains(Features::MULTI_DRAW_INDIRECT)
      && info
        .supported_features
        .contains(Features::INDIRECT_FIRST_INSTANCE)
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

    if bindless_effectively_supported {
      return None;
    }

    let inner = GPUBindlessMeshSystemInner {
      any_changed: Default::default(),
      meta_data: Default::default(),
      index_buffer: GPUSubAllocateBuffer::init_with_initial_item_count(
        &gpu.device,
        10_0000,
        1000_0000,
        BufferUsages::INDEX,
      ),
      position_vertex_buffers: Default::default(),
      normal_vertex_buffers: Default::default(),
      uv_vertex_buffers: Default::default(),
      bindless_position_vertex_buffers: Default::default(),
      bindless_normal_vertex_buffers: Default::default(),
      bindless_uv_vertex_buffers: Default::default(),
    };

    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
    .into()
  }

  pub fn maintain(&mut self) {
    let mut inner = self.inner.write().unwrap();
    if !inner.any_changed {
      return;
    }

    todo!();

    inner.any_changed = false;
  }

  /// maybe unable to allocate more!
  pub fn create_mesh_instance(
    &mut self,
    index: Vec<u32>,
    position: Vec<Vec3<f32>>,
    normal: Vec<Vec3<f32>>,
    uv: Vec<Vec2<f32>>,
    device: &GPUDevice,
    queue: &GPUQueue,
  ) -> Option<MeshSystemMeshInstance> {
    assert_eq!(position.len(), normal.len());
    assert_eq!(position.len(), uv.len());

    let mut inner = self.inner.write().unwrap();
    inner.any_changed = true;

    let position = StorageBufferReadOnlyDataView::create(device, position.as_slice());
    let normal = StorageBufferReadOnlyDataView::create(device, normal.as_slice());
    let uv = StorageBufferReadOnlyDataView::create(device, uv.as_slice());

    let metadata = DrawMetaData {
      start: 0, // todo
      count: index.len() as u32,
      vertex_info: DrawVertexIndirectInfo {
        position_buffer_id: inner.position_vertex_buffers.insert(position) as u32,
        normal_buffer_id: inner.normal_vertex_buffers.insert(normal) as u32,
        uv_buffer_id: inner.uv_vertex_buffers.insert(uv) as u32,
        ..Zeroable::zeroed()
      },
      ..Zeroable::zeroed()
    };
    let handle = inner.meta_data.insert(metadata) as u32;

    MeshSystemMeshInstance {
      handle,
      _index_holder: inner.index_buffer.allocate(&index, device, queue)?,
      system: Arc::downgrade(&self.inner),
    }
    .into()
  }
}

// impl Stream for GPUBindlessMeshSystem {

// }

#[derive(Clone)]
pub struct MeshSystemMeshInstance {
  handle: MeshSystemMeshHandle,
  _index_holder: GPUSubAllocateBufferToken<u32>,
  system: Weak<RwLock<GPUBindlessMeshSystemInner>>,
}

impl MeshSystemMeshInstance {
  pub fn mesh_handle(&self) -> MeshSystemMeshHandle {
    self.handle
  }
}

impl Drop for MeshSystemMeshInstance {
  fn drop(&mut self) {
    if let Some(system) = self.system.upgrade() {
      let mut system = system.write().unwrap();
      system.any_changed = true;

      let meta = system.meta_data.remove(self.handle as usize);
      let vertex = meta.vertex_info;
      system
        .position_vertex_buffers
        .remove(vertex.position_buffer_id as usize);
      system
        .normal_vertex_buffers
        .remove(vertex.normal_buffer_id as usize);
      system
        .uv_vertex_buffers
        .remove(vertex.uv_buffer_id as usize);
    }
  }
}
