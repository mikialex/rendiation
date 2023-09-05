#![feature(strict_provenance)]

use std::sync::{Arc, RwLock, Weak};

use rendiation_shader_api::*;
use rendiation_webgpu::*;
use slab::Slab;

mod allocator;
use allocator::*;

mod wrap;
pub use wrap::*;

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

type BindlessPositionVertexBuffer = BindingResourceArray<
  StorageBufferReadOnlyDataView<[Vec3<f32>]>,
  MAX_STORAGE_BINDING_ARRAY_LENGTH,
>;

type BindlessNormalVertexBuffer = BindlessPositionVertexBuffer;

type BindlessUvVertexBuffer = BindingResourceArray<
  StorageBufferReadOnlyDataView<[Vec2<f32>]>,
  MAX_STORAGE_BINDING_ARRAY_LENGTH,
>;

pub struct GPUBindlessMeshSystemInner {
  any_changed: bool,
  metadata: Slab<DrawMetaData>,

  index_buffer: GPUSubAllocateBuffer,
  relocations: Arc<RwLock<Vec<RelocationMessage>>>, // we could use a channel, so what?

  position_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec3<f32>]>>,
  normal_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec3<f32>]>>,
  uv_vertex_buffers: Slab<StorageBufferReadOnlyDataView<[Vec2<f32>]>>,

  bindless_position_vertex_buffers: BindlessPositionVertexBuffer,
  bindless_normal_vertex_buffers: BindlessNormalVertexBuffer,
  bindless_uv_vertex_buffers: BindlessUvVertexBuffer,
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

    if !bindless_effectively_supported {
      return None;
    }

    let index_buffer = GPUSubAllocateBuffer::init_with_initial_item_count(
      &gpu.device,
      10_0000,
      1000_0000,
      BufferUsages::INDEX,
    );

    let relocations: Arc<RwLock<Vec<RelocationMessage>>> = Default::default();

    let r = relocations.clone();
    index_buffer.set_relocate_callback(move |m| r.write().unwrap().push(m));

    let inner = GPUBindlessMeshSystemInner {
      any_changed: Default::default(),
      metadata: Default::default(),
      index_buffer,
      relocations,
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
    let inner: &mut GPUBindlessMeshSystemInner = &mut inner;
    if !inner.any_changed {
      return;
    }

    {
      let metadata = &mut inner.metadata;
      let relocations = &mut inner.relocations.write().unwrap();
      let relocations: &mut Vec<RelocationMessage> = relocations;
      relocations.iter().for_each(|m| {
        let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
        meta.start = m.new_offset;
      });
      *relocations = Vec::new(); // free any space
    }

    let source = slab_to_vec(&inner.position_vertex_buffers);
    inner.bindless_position_vertex_buffers = BindlessPositionVertexBuffer::new(Arc::new(source));

    let source = slab_to_vec(&inner.normal_vertex_buffers);
    inner.bindless_normal_vertex_buffers = BindlessNormalVertexBuffer::new(Arc::new(source));

    let source = slab_to_vec(&inner.uv_vertex_buffers);
    inner.bindless_uv_vertex_buffers = BindlessUvVertexBuffer::new(Arc::new(source));

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
      start: 0, // todo, currently we not effectively support this.
      count: index.len() as u32,
      vertex_info: DrawVertexIndirectInfo {
        position_buffer_id: inner.position_vertex_buffers.insert(position) as u32,
        normal_buffer_id: inner.normal_vertex_buffers.insert(normal) as u32,
        uv_buffer_id: inner.uv_vertex_buffers.insert(uv) as u32,
        ..Zeroable::zeroed()
      },
      ..Zeroable::zeroed()
    };
    let handle = inner.metadata.insert(metadata) as u32;

    MeshSystemMeshInstance {
      handle,
      _index_holder: Arc::new(inner.index_buffer.allocate(
        handle,
        bytemuck::cast_slice(&index),
        device,
        queue,
      )?),
      system: Arc::downgrade(&self.inner),
    }
    .into()
  }
}

#[derive(Clone)]
pub struct MeshSystemMeshInstance {
  handle: MeshSystemMeshHandle,
  _index_holder: Arc<GPUSubAllocateBufferToken>,
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

      let meta = system.metadata.remove(self.handle as usize);
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

// this is not good, maybe we should impl slab by ourself?
fn slab_to_vec<T: Clone>(s: &Slab<T>) -> Vec<T> {
  let mut r = Vec::with_capacity(s.capacity());
  let default = s.get(0).unwrap();
  s.iter().for_each(|(idx, v)| {
    while idx >= r.len() {
      r.push(default.clone())
    }
    r[idx] = v.clone();
  });
  r
}
