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

pub struct GPUBindlessMeshSystem {
  metadata: Slab<DrawMetaData>,

  index_buffer: GPURangeAllocateBuffer,

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

    let index_buffer = GPURangeAllocateBuffer::init_with_initial_item_count(
      &gpu.device,
      10_0000,
      1000_0000,
      std::mem::size_of::<u32>(),
      BufferUsages::INDEX,
    );

    let vertex_init_count = 10_0000;
    let vertex_max_count = 1000_0000;

    let position =
      StorageBufferRangeAllocatePool::new(&gpu.device, vertex_init_count, vertex_max_count);
    let normal =
      StorageBufferRangeAllocatePool::new(&gpu.device, vertex_init_count, vertex_max_count);
    let uv = StorageBufferRangeAllocatePool::new(&gpu.device, vertex_init_count, vertex_max_count);

    GPUBindlessMeshSystem {
      metadata: Default::default(),
      index_buffer,
      position,
      normal,
      uv,
    }
    .into()
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
    let index_alloc = self
      .index_buffer
      .allocate(handle, index, device, queue, &mut |m| {
        let meta = self.metadata.get_mut(m.allocation_handle as usize).unwrap();
        meta.start = m.new_offset;
      })?;

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

    let position_alloc = self
      .position
      .allocate(handle, position, device, queue, &mut |m| {
        let meta = self.metadata.get_mut(m.allocation_handle as usize).unwrap();
        meta.vertex_info.position_buffer_offset = m.new_offset;
      })?;
    let normal_alloc = self
      .normal
      .allocate(handle, normal, device, queue, &mut |m| {
        let meta = self.metadata.get_mut(m.allocation_handle as usize).unwrap();
        meta.vertex_info.normal_buffer_offset = m.new_offset;
      })?;
    let uv_alloc = self.uv.allocate(handle, uv, device, queue, &mut |m| {
      let meta = self.metadata.get_mut(m.allocation_handle as usize).unwrap();
      meta.vertex_info.uv_buffer_offset = m.new_offset;
    })?;

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
