use fast_hash_collection::FastHashMap;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use slab::Slab;

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

pub struct BindlessMeshSource<'a> {
  pub index: &'a [u32],
  pub position: &'a [Vec4<f32>],
  pub normal: &'a [Vec4<f32>],
  pub uv: &'a [Vec2<f32>],
}

pub struct GPUBindlessMeshSystem {
  metadata: Slab<DrawMetaData>,
  position_offset: FastHashMap<u32, u32>,
  normal_offset: FastHashMap<u32, u32>,
  uv_offset: FastHashMap<u32, u32>,
  index_offset: FastHashMap<u32, u32>,

  index_buffer: RangeAllocatePool<TypedGPUBuffer<u32>>,

  position: StorageBufferRangeAllocatePool<Vec4<f32>>,
  normal: StorageBufferRangeAllocatePool<Vec4<f32>>,
  uv: StorageBufferRangeAllocatePool<Vec4<f32>>,
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

    let usage = BufferUsages::INDEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
    let init_size = 10_0000;
    let index_buffer = TypedGPUBuffer::new(
      create_gpu_buffer_zeroed(init_size as u64 * 4, usage, &gpu.device).create_default_view(),
    );

    let index_buffer = create_growable_buffer(gpu, index_buffer, 1000_0000);
    let index_buffer = GPURangeAllocateMaintainer::new(gpu, index_buffer);

    let vertex_init_count = 10_0000;
    let vertex_max_count = 1000_0000;

    let position =
      create_storage_buffer_range_allocate_pool(gpu, vertex_init_count, vertex_max_count);
    let normal =
      create_storage_buffer_range_allocate_pool(gpu, vertex_init_count, vertex_max_count);
    let uv = create_storage_buffer_range_allocate_pool(gpu, vertex_init_count, vertex_max_count);

    GPUBindlessMeshSystem {
      metadata: Default::default(),
      index_buffer,
      position,
      normal,
      uv,
      position_offset: Default::default(),
      normal_offset: Default::default(),
      uv_offset: Default::default(),
      index_offset: Default::default(),
    }
    .into()
  }

  /// maybe unable to allocate more!
  pub fn create_mesh_instance(
    &mut self,
    source: BindlessMeshSource,
  ) -> Option<MeshSystemMeshInstance> {
    let BindlessMeshSource {
      index,
      position,
      normal,
      uv,
    } = source;

    assert_eq!(position.len(), normal.len());
    assert_eq!(position.len(), uv.len());

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
    let index_alloc = self.index_buffer.allocate_values(index, &mut |m| {
      let meta_handle = self.index_offset.remove(&m.previous_offset).unwrap();
      let meta = self.metadata.get_mut(meta_handle as usize).unwrap();
      meta.start = m.new_offset;
      self.index_offset.insert(m.new_offset, meta_handle).unwrap();
    })?;
    self.index_offset.insert(index_alloc, handle);

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

    let position_alloc = self.position.allocate_values(position, &mut |m| {
      let meta_handle = self.position_offset.remove(&m.previous_offset).unwrap();
      let meta = self.metadata.get_mut(meta_handle as usize).unwrap();
      meta.vertex_info.position_buffer_offset = m.new_offset;
      self
        .position_offset
        .insert(m.new_offset, meta_handle)
        .unwrap();
    })?;
    self.position_offset.insert(position_alloc, handle);

    let normal_alloc = self.normal.allocate_values(normal, &mut |m| {
      let meta_handle = self.normal_offset.remove(&m.previous_offset).unwrap();
      let meta = self.metadata.get_mut(meta_handle as usize).unwrap();
      meta.vertex_info.normal_buffer_offset = m.new_offset;
      self
        .normal_offset
        .insert(m.new_offset, meta_handle)
        .unwrap();
    })?;
    self.normal_offset.insert(position_alloc, handle);

    let uv_alloc = self.uv.allocate_values(uv, &mut |m| {
      let meta_handle = self.uv_offset.remove(&m.previous_offset).unwrap();
      let meta = self.metadata.get_mut(meta_handle as usize).unwrap();
      meta.vertex_info.uv_buffer_offset = m.new_offset;
      self.uv_offset.insert(m.new_offset, meta_handle).unwrap();
    })?;
    self.uv_offset.insert(position_alloc, handle);

    let metadata = self.metadata.get_mut(handle as usize).unwrap();
    metadata.start = index_alloc;
    metadata.vertex_info.position_buffer_offset = position_alloc;
    metadata.vertex_info.normal_buffer_offset = normal_alloc;
    metadata.vertex_info.uv_buffer_offset = uv_alloc;

    MeshSystemMeshInstance { handle }.into()
  }

  pub fn remove_mesh_instance(&mut self, instance: MeshSystemMeshInstance) {
    self.metadata.remove(instance.handle as usize);
    self.index_buffer.deallocate(instance.handle);
    self.position.deallocate(instance.handle);
    self.normal.deallocate(instance.handle);
    self.uv.deallocate(instance.handle);
  }
}

pub struct MeshSystemMeshInstance {
  handle: MeshSystemMeshHandle,
}

impl MeshSystemMeshInstance {
  pub fn mesh_handle(&self) -> MeshSystemMeshHandle {
    self.handle
  }
}
