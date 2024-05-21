use core::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
};
use std::sync::{Arc, RwLock, Weak};

use rendiation_shader_api::*;
use rendiation_webgpu::*;
use slab::Slab;

mod allocator;
use allocator::*;

mod draw;
pub use draw::*;

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
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct DrawVertexIndirectInfo {
  pub position_buffer_offset: u32,
  pub normal_buffer_offset: u32,
  pub uv_buffer_offset: u32,
}

#[repr(C)]
#[derive(Clone, Copy, ShaderStruct, Zeroable, Pod, Debug)]
pub struct DrawIndexedIndirect {
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

#[derive(Clone)]
pub struct GPUBindlessMeshSystem {
  inner: Arc<RwLock<GPUBindlessMeshSystemImpl>>,
}

pub struct BindlessMeshSource<'a> {
  pub index: &'a [u32],
  pub position: &'a [Vec4<f32>],
  pub normal: &'a [Vec4<f32>],
  pub uv: &'a [Vec2<f32>],
}

pub struct BufferPool {
  buffer: GPUSubAllocateBuffer,
  // we could use a channel, so what?
  relocations: Arc<RwLock<Vec<RelocationMessage>>>,
}

impl Deref for BufferPool {
  type Target = GPUSubAllocateBuffer;

  fn deref(&self) -> &Self::Target {
    &self.buffer
  }
}
impl DerefMut for BufferPool {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.buffer
  }
}

impl BufferPool {
  pub fn new(
    init_byte: usize,
    max_byte: usize,
    usage: BufferUsages,
    item_byte_size: usize,
    device: &GPUDevice,
  ) -> Self {
    let buffer = GPUSubAllocateBuffer::init_with_initial_item_count(
      device,
      init_byte,
      max_byte,
      item_byte_size,
      usage,
    );

    let relocations: Arc<RwLock<Vec<RelocationMessage>>> = Default::default();

    let r = relocations.clone();
    buffer.set_relocate_callback(move |m| r.write().unwrap().push(m));

    Self {
      buffer,
      relocations,
    }
  }

  pub fn flush_relocation(&self, cb: impl FnMut(&RelocationMessage)) {
    let mut relocations = self.relocations.write().unwrap();
    let relocations: &mut Vec<RelocationMessage> = &mut relocations;
    relocations.iter().for_each(cb);
    *relocations = Vec::new(); // free any space
  }
}

pub struct VertexBufferPool<T> {
  pool: BufferPool,
  ty: PhantomData<T>,
}

impl<T> Deref for VertexBufferPool<T> {
  type Target = BufferPool;

  fn deref(&self) -> &Self::Target {
    &self.pool
  }
}
impl<T> DerefMut for VertexBufferPool<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pool
  }
}

impl<T> VertexBufferPool<T> {
  pub fn new(pool: BufferPool) -> Self {
    Self {
      pool,
      ty: Default::default(),
    }
  }
}

impl<T> CacheAbleBindingSource for VertexBufferPool<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.pool.buffer.get_buffer().get_binding_build_source()
  }
}

impl<T: ShaderSizedValueNodeType> ShaderBindingProvider for VertexBufferPool<T> {
  type Node = ShaderReadOnlyStoragePtr<[T]>;
}

pub struct GPUBindlessMeshSystemImpl {
  any_changed: bool,
  metadata: Slab<DrawMetaData>,

  index_buffer: BufferPool,

  position: VertexBufferPool<Vec4<f32>>,
  normal: VertexBufferPool<Vec4<f32>>,
  uv: VertexBufferPool<Vec4<f32>>,
}

impl GPUBindlessMeshSystem {
  pub fn new(gpu: &GPU) -> Option<Self> {
    let info = gpu.info();
    let bindless_effectively_supported = info
      .supported_features
      .contains(Features::MULTI_DRAW_INDIRECT)
      && info
        .supported_features
        .contains(Features::INDIRECT_FIRST_INSTANCE);

    if !bindless_effectively_supported {
      return None;
    }

    let index_buffer = BufferPool::new(10_0000, 1000_0000, BufferUsages::INDEX, 4, &gpu.device);

    let position = BufferPool::new(
      10_0000,
      1000_0000,
      BufferUsages::STORAGE,
      4 * 4,
      &gpu.device,
    );
    let position = VertexBufferPool::new(position);

    let normal = BufferPool::new(
      10_0000,
      1000_0000,
      BufferUsages::STORAGE,
      4 * 4,
      &gpu.device,
    );
    let normal = VertexBufferPool::new(normal);

    let uv = BufferPool::new(
      10_0000,
      1000_0000,
      BufferUsages::STORAGE,
      4 * 4,
      &gpu.device,
    );
    let uv = VertexBufferPool::new(uv);

    let inner = GPUBindlessMeshSystemImpl {
      any_changed: true,
      metadata: Default::default(),
      index_buffer,
      position,
      normal,
      uv,
    };

    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
    .into()
  }

  pub fn maintain(&mut self) {
    let mut inner = self.inner.write().unwrap();
    let inner: &mut GPUBindlessMeshSystemImpl = &mut inner;
    if !inner.any_changed {
      return;
    }

    let metadata = &mut inner.metadata;

    inner.index_buffer.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.start = m.new_offset;
    });

    inner.position.pool.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.vertex_info.position_buffer_offset = m.new_offset;
    });

    inner.normal.pool.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.vertex_info.normal_buffer_offset = m.new_offset;
    });

    inner.uv.pool.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.vertex_info.uv_buffer_offset = m.new_offset;
    });

    inner.any_changed = false;
  }

  /// maybe unable to allocate more!
  pub fn create_mesh_instance(
    &self,
    source: BindlessMeshSource,
    device: &GPUDevice,
    queue: &GPUQueue,
  ) -> Option<MeshSystemMeshInstance> {
    let BindlessMeshSource {
      index,
      position,
      normal,
      uv,
    } = source;

    assert_eq!(position.len(), normal.len());
    assert_eq!(position.len(), uv.len());

    let mut inner = self.inner.write().unwrap();
    inner.any_changed = true;

    // will write later..
    let metadata = DrawMetaData {
      start: 0,
      count: index.len() as u32,
      vertex_info: DrawVertexIndirectInfo {
        ..Zeroable::zeroed()
      },
      ..Zeroable::zeroed()
    };
    let handle = inner.metadata.insert(metadata) as u32;

    let index = bytemuck::cast_slice(index);
    let (allocation, start) = inner.index_buffer.allocate(handle, index, device, queue)?;

    let position: Vec<Vec4<f32>> = position
      .iter()
      .map(|v| Vec4::new(v.x, v.y, v.z, 0.))
      .collect();
    let normal: Vec<Vec4<f32>> = normal
      .iter()
      .map(|v| Vec4::new(v.x, v.y, v.z, 0.))
      .collect();
    let uv: Vec<Vec4<f32>> = uv.iter().map(|v| Vec4::new(v.x, v.y, 0., 0.)).collect();

    let position = bytemuck::cast_slice(&position);
    let normal = bytemuck::cast_slice(&normal);
    let uv = bytemuck::cast_slice(&uv);

    let (_position_holder, position) = inner.position.allocate(handle, position, device, queue)?;
    let (_normal_holder, normal) = inner.normal.allocate(handle, normal, device, queue)?;
    let (_uv_holder, uv) = inner.uv.allocate(handle, uv, device, queue)?;

    let metadata = inner.metadata.get_mut(handle as usize).unwrap();
    metadata.start = start;
    metadata.vertex_info.position_buffer_offset = position;
    metadata.vertex_info.normal_buffer_offset = normal;
    metadata.vertex_info.uv_buffer_offset = uv;

    MeshSystemMeshInstance {
      handle,
      _index_holder: Arc::new(allocation),
      _uv_holder: Arc::new(_uv_holder),
      _position_holder: Arc::new(_position_holder),
      _normal_holder: Arc::new(_normal_holder),
      system: Arc::downgrade(&self.inner),
    }
    .into()
  }
}

pub struct MeshSystemMeshInstance {
  handle: MeshSystemMeshHandle,
  _index_holder: Arc<GPUSubAllocateBufferToken>,
  _position_holder: Arc<GPUSubAllocateBufferToken>,
  _uv_holder: Arc<GPUSubAllocateBufferToken>,
  _normal_holder: Arc<GPUSubAllocateBufferToken>,
  system: Weak<RwLock<GPUBindlessMeshSystemImpl>>,
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

      system.metadata.remove(self.handle as usize);
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
