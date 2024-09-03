use core::{
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

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

pub struct BindlessMeshSource<'a> {
  pub index: &'a [u32],
  pub position: &'a [Vec4<f32>],
  pub normal: &'a [Vec4<f32>],
  pub uv: &'a [Vec2<f32>],
}

pub struct BufferPool {
  buffer: GPUSubAllocateBuffer,
  // we could instead use a channel here
  relocations: Vec<RelocationMessage>,
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

    Self {
      buffer,
      relocations: Default::default(),
    }
  }

  pub fn allocate(
    &mut self,
    allocation_handle: u32,
    content: &[u8],
    device: &GPUDevice,
    queue: &GPUQueue,
  ) -> Option<GPUSubAllocateResult> {
    self.buffer.allocate(
      allocation_handle,
      content,
      device,
      queue,
      &mut |relocation| self.relocations.push(relocation),
    )
  }

  pub fn deallocate(&mut self, token: u32) {
    self.buffer.deallocate(token)
  }

  pub fn flush_relocation(&mut self, cb: impl FnMut(&RelocationMessage)) {
    self.relocations.iter().for_each(cb);
    self.relocations = Vec::new(); // free any space
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
    self.pool.buffer.buffer().get_binding_build_source()
  }
}

impl<T: ShaderSizedValueNodeType> ShaderBindingProvider for VertexBufferPool<T> {
  type Node = ShaderReadOnlyStoragePtr<[T]>;
}

pub struct GPUBindlessMeshSystem {
  any_changed: bool,
  metadata: Slab<DrawMetaData>,

  index_buffer: BufferPool,

  position: VertexBufferPool<Vec4<f32>>,
  normal: VertexBufferPool<Vec4<f32>>,
  uv: VertexBufferPool<Vec4<f32>>,
}

impl GPUBindlessMeshSystem {
  pub fn new(gpu: &GPU) -> Option<Self> {
    let bindless_effectively_supported = gpu
      .info
      .supported_features
      .contains(Features::MULTI_DRAW_INDIRECT)
      && gpu
        .info
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

    GPUBindlessMeshSystem {
      any_changed: true,
      metadata: Default::default(),
      index_buffer,
      position,
      normal,
      uv,
    }
    .into()
  }

  pub fn maintain(&mut self) {
    if !self.any_changed {
      return;
    }

    let metadata = &mut self.metadata;

    self.index_buffer.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.start = m.new_offset;
    });

    self.position.pool.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.vertex_info.position_buffer_offset = m.new_offset;
    });

    self.normal.pool.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.vertex_info.normal_buffer_offset = m.new_offset;
    });

    self.uv.pool.flush_relocation(|m| {
      let meta = metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.vertex_info.uv_buffer_offset = m.new_offset;
    });

    self.any_changed = false;
  }

  /// maybe unable to allocate more!
  pub fn create_mesh_instance(
    &mut self,
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

    self.any_changed = true;

    // will write later..
    let metadata = DrawMetaData {
      start: 0,
      count: index.len() as u32,
      vertex_info: DrawVertexIndirectInfo {
        ..Zeroable::zeroed()
      },
      ..Zeroable::zeroed()
    };
    let handle = self.metadata.insert(metadata) as u32;

    let index = bytemuck::cast_slice(index);
    let index_alloc = self.index_buffer.allocate(handle, index, device, queue)?;

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

    let position_alloc = self.position.allocate(handle, position, device, queue)?;
    let normal_alloc = self.normal.allocate(handle, normal, device, queue)?;
    let uv_alloc = self.uv.allocate(handle, uv, device, queue)?;

    let metadata = self.metadata.get_mut(handle as usize).unwrap();
    metadata.start = index_alloc.allocate_offset;
    metadata.vertex_info.position_buffer_offset = position_alloc.allocate_offset;
    metadata.vertex_info.normal_buffer_offset = normal_alloc.allocate_offset;
    metadata.vertex_info.uv_buffer_offset = uv_alloc.allocate_offset;

    MeshSystemMeshInstance {
      handle,
      index_token: index_alloc.token,
      position_token: position_alloc.token,
      normal_token: normal_alloc.token,
      uv_token: uv_alloc.token,
    }
    .into()
  }

  pub fn remove_mesh_instance(&mut self, instance: MeshSystemMeshInstance) {
    self.any_changed = true;

    self.metadata.remove(instance.handle as usize);
    self.index_buffer.deallocate(instance.index_token);
    self.position.deallocate(instance.position_token);
    self.normal.deallocate(instance.normal_token);
    self.uv.deallocate(instance.uv_token);
  }
}

pub struct MeshSystemMeshInstance {
  handle: MeshSystemMeshHandle,
  index_token: u32,
  position_token: u32,
  normal_token: u32,
  uv_token: u32,
}

impl MeshSystemMeshInstance {
  pub fn mesh_handle(&self) -> MeshSystemMeshHandle {
    self.handle
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
