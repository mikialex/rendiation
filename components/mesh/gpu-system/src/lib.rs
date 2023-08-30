use __core::ops::Range;
use fast_hash_collection::FastHashMap;
use rendiation_shader_api::*;
use slab::Slab;

// todo, support runtime size by query client limitation
pub const MAX_STORAGE_BINDING_ARRAY_LENGTH: usize = 8192;

pub trait GPUMeshBackend {
  type BindingCollector;

  type GPUIndexBuffer;
  type GPUStorageBuffer<T>: ShaderBindingProvider<Node = ShaderReadOnlyStoragePtr<[T]>> + Clone;
  type GPUStorageBufferBindingArray<T, const N: usize>: ShaderBindingProvider<
      Node = ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderReadOnlyStoragePtr<[T]>>, N>>,
    > + Default;

  fn bind_storage<T>(collector: &mut Self::BindingCollector, sampler: &Self::GPUStorageBuffer<T>);
  fn bind_storage_array<T, const N: usize>(
    collector: &mut Self::BindingCollector,
    textures: &Self::GPUStorageBufferBindingArray<T, N>,
  );
}

#[derive(Clone, Copy)]
pub struct MeshSystemMeshHandle {
  pub inner: u32,
}

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

struct MeshGPUDrivenSystemInner {}

pub struct MeshGPUDrivenSystem<B: GPUMeshBackend> {
  next_id: usize,
  // range to index buffer,index to vertex_indirect_buffer
  draw_ranges: FastHashMap<u32, Range<u32>>,

  index_buffer: B::GPUIndexBuffer,

  position_vertex_buffers: Slab<B::GPUStorageBuffer<Vec3<f32>>>,
  normal_vertex_buffers: Slab<B::GPUStorageBuffer<Vec3<f32>>>,
  normal_uv_buffers: Slab<B::GPUStorageBuffer<Vec2<f32>>>,

  bindless_position_vertex_buffers_f32:
    B::GPUStorageBufferBindingArray<Vec3<f32>, MAX_STORAGE_BINDING_ARRAY_LENGTH>,
  bindless_normal_vertex_buffers_u32:
    B::GPUStorageBufferBindingArray<Vec3<f32>, MAX_STORAGE_BINDING_ARRAY_LENGTH>,
  bindless_uv_vertex_buffers_u32:
    B::GPUStorageBufferBindingArray<Vec2<f32>, MAX_STORAGE_BINDING_ARRAY_LENGTH>,
}

// impl Stream for MeshGPUDrivenSystem {

// }

#[repr(C)]
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

impl<B: GPUMeshBackend> MeshGPUDrivenSystem<B> {
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
    buffer: impl Iterator<Item = MeshSystemMeshHandle>,
  ) -> impl Iterator<Item = (DrawIndirect, DrawVertexIndirectInfo)> {
    [].into_iter() // todo
  }

  /// user could use this in their compute shader to generate the buffer we want
  pub fn generate_draw_command_on_device(
    &self,
    handle: Node<u32>,
  ) -> (Node<DrawIndirect>, Node<DrawVertexIndirectInfo>) {
    todo!()
  }

  // todo, check how to get draw_id https://www.g-truc.net/post-0518.html
  pub fn fetch_vertex_on_device(
    &self,
    draw_id: Node<u32>,
    vertex_id: Node<u32>,
    info: Node<DrawVertexIndirectInfo>,
  ) -> Node<Vertex> {
    todo!()
  }
}
