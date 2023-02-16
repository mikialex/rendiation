use bytemuck::Pod;
use core::marker::PhantomData;
use rendiation_renderable_mesh::{GroupedMesh, IndexGet, MeshGroup};
use shadergraph::*;
use webgpu::DrawCommand;

use crate::*;

pub struct MeshGPU {
  range_full: MeshGroup,
  vertex: Vec<GPUBufferResourceView>,
  index: Option<(GPUBufferResourceView, webgpu::IndexFormat)>,
}

pub struct TypedMeshGPU<T> {
  marker: PhantomData<T>,
  inner: MeshGPU,
}

impl<V, T, IU> ShaderGraphProvider for TypedMeshGPU<GroupedMesh<IndexedMesh<T, Vec<V>, IU>>>
where
  V: ShaderGraphVertexInProvider,
  T: PrimitiveTopologyMeta,
{
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      builder.register_vertex::<V>(VertexStepMode::Vertex);
      builder.primitive_state.topology = map_topology(T::ENUM);
      Ok(())
    })
  }
}

impl<T> webgpu::ShaderPassBuilder for TypedMeshGPU<T> {
  fn setup_pass(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    self.setup_pass(ctx)
  }
}

/// variance info is encoded in T's type id
impl<T: 'static> webgpu::ShaderHashProvider for TypedMeshGPU<T> {}

impl MeshGPU {
  pub fn get_range_full(&self) -> MeshGroup {
    self.range_full
  }

  pub fn setup_pass(&self, pass: &mut GPURenderPassCtx) {
    self.vertex.iter().for_each(|gpu| {
      pass.set_vertex_buffer_owned_next(gpu);
    });
    if let Some((index, format)) = &self.index {
      pass.pass.set_index_buffer_owned(index, *format);
    }
  }
}

impl<T> TypedMeshGPU<T> {
  pub fn get_range_full(&self) -> MeshGroup {
    self.inner.get_range_full()
  }

  pub fn setup_pass(&self, pass: &mut GPURenderPassCtx) {
    self.inner.setup_pass(pass)
  }
}

pub trait IndexBufferSourceTypeProvider {
  fn format(&self) -> webgpu::IndexFormat;
}

impl<T: IndexBufferSourceType> IndexBufferSourceTypeProvider for Vec<T> {
  fn format(&self) -> webgpu::IndexFormat {
    T::FORMAT
  }
}
impl<T: IndexBufferSourceType> IndexBufferSourceTypeProvider for IndexBuffer<T> {
  fn format(&self) -> webgpu::IndexFormat {
    T::FORMAT
  }
}
impl IndexBufferSourceTypeProvider for DynIndexContainer {
  fn format(&self) -> webgpu::IndexFormat {
    match self {
      DynIndexContainer::Uint16(_) => u16::FORMAT,
      DynIndexContainer::Uint32(_) => u32::FORMAT,
    }
  }
}

impl<V, T, IU> WebGPUMesh for GroupedMesh<IndexedMesh<T, Vec<V>, IU>>
where
  V: Pod,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider + 'static,
  V: ShaderGraphVertexInProvider,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
  T: PrimitiveTopologyMeta,
  Self: IntersectAbleGroupedMesh,
  Self: SimpleIncremental,
  Self: Send + Sync,
{
  type GPU = TypedMeshGPU<Self>;
  fn create(&self, gpu: &GPU, _storage: &mut AnyMap) -> Self::GPU {
    TypedMeshGPU {
      marker: Default::default(),
      inner: create_gpu(&self.mesh, &gpu.device),
    }
  }
  fn update(&self, g: &mut Self::GPU, gpu: &GPU, storage: &mut AnyMap) {
    *g = self.create(gpu, storage)
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    let range = self.get_group(group);
    DrawCommand::Indexed {
      base_vertex: 0,
      indices: range.into(),
      instances: 0..1,
    }
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    map_topology(T::ENUM)
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    f(self)
  }
}

pub fn map_topology(pt: PrimitiveTopology) -> webgpu::PrimitiveTopology {
  match pt {
    PrimitiveTopology::PointList => webgpu::PrimitiveTopology::PointList,
    PrimitiveTopology::LineList => webgpu::PrimitiveTopology::LineList,
    PrimitiveTopology::LineStrip => webgpu::PrimitiveTopology::LineStrip,
    PrimitiveTopology::TriangleList => webgpu::PrimitiveTopology::TriangleList,
    PrimitiveTopology::TriangleStrip => webgpu::PrimitiveTopology::TriangleStrip,
  }
}

pub trait AsGPUBytes {
  fn as_gpu_bytes(&self) -> &[u8];
}

impl<T: Pod> AsGPUBytes for Vec<T> {
  fn as_gpu_bytes(&self) -> &[u8] {
    bytemuck::cast_slice(self.as_slice())
  }
}

impl AsGPUBytes for DynIndexContainer {
  fn as_gpu_bytes(&self) -> &[u8] {
    match self {
      DynIndexContainer::Uint16(i) => bytemuck::cast_slice(i.as_slice()),
      DynIndexContainer::Uint32(i) => bytemuck::cast_slice(i.as_slice()),
    }
  }
}

pub fn create_gpu<V, T, IU>(
  mesh: &IndexedMesh<T, Vec<V>, IU>,
  device: &webgpu::GPUDevice,
) -> MeshGPU
where
  V: Pod,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
{
  let vertex = bytemuck::cast_slice(mesh.vertex.as_slice());
  let vertex =
    create_gpu_buffer(vertex, webgpu::BufferUsages::VERTEX, device).create_default_view();

  let vertex = vec![vertex];

  let index = create_gpu_buffer(
    mesh.index.as_gpu_bytes(),
    webgpu::BufferUsages::INDEX,
    device,
  )
  .create_default_view();

  let index = (index, mesh.index.format()).into();

  let range_full = MeshGroup {
    start: 0,
    count: mesh.draw_count(),
  };

  MeshGPU {
    vertex,
    index,
    range_full,
  }
}
