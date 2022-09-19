use std::rc::Rc;

use bytemuck::Pod;
use core::marker::PhantomData;
use gpu::util::DeviceExt;
use gpu::DrawCommand;
use gpu::GPURenderPassCtx;
use gpu::IndexBufferSourceType;
use rendiation_webgpu as gpu;
use shadergraph::*;

use crate::*;

pub struct MeshGPU {
  range_full: MeshGroup,
  vertex: Vec<Rc<gpu::Buffer>>,
  index: Option<(Rc<gpu::Buffer>, gpu::IndexFormat)>,
}

pub struct TypedMeshGPU<T> {
  marker: PhantomData<T>,
  inner: MeshGPU,
}

impl<T: GPUMeshData> ShaderGraphProvider for TypedMeshGPU<T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    T::build_shader(builder);
    Ok(())
  }
}

impl<T> gpu::ShaderPassBuilder for TypedMeshGPU<T> {
  fn setup_pass(&self, ctx: &mut gpu::GPURenderPassCtx) {
    self.setup_pass(ctx)
  }
}

impl<T: 'static> gpu::ShaderHashProvider for TypedMeshGPU<T> {}

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

/// The GPUMesh's cpu data source trait
pub trait GPUMeshData {
  type GPU;
  fn create(&self, device: &gpu::Device) -> Self::GPU;
  fn update(&self, gpu: &mut Self::GPU, device: &gpu::Device);
  fn get_group(&self, group: MeshDrawGroup) -> MeshGroup;
  fn topology(&self) -> gpu::PrimitiveTopology;
  fn draw(&self, group: MeshDrawGroup) -> DrawCommand;

  fn build_shader(builder: &mut ShaderGraphRenderPipelineBuilder);
}

pub trait IndexBufferSourceTypeProvider {
  fn format(&self) -> gpu::IndexFormat;
}

impl<T: IndexBufferSourceType> IndexBufferSourceTypeProvider for Vec<T> {
  fn format(&self) -> gpu::IndexFormat {
    T::FORMAT
  }
}
impl<T: IndexBufferSourceType> IndexBufferSourceTypeProvider for IndexBuffer<T> {
  fn format(&self) -> gpu::IndexFormat {
    T::FORMAT
  }
}
impl IndexBufferSourceTypeProvider for DynIndexContainer {
  fn format(&self) -> gpu::IndexFormat {
    match self {
      DynIndexContainer::Uint16(_) => u16::FORMAT,
      DynIndexContainer::Uint32(_) => u32::FORMAT,
    }
  }
}

impl<V, T, IU> GPUMeshData for GroupedMesh<IndexedMesh<T, Vec<V>, IU>>
where
  V: Pod,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider,
  V: ShaderGraphVertexInProvider,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
  T: PrimitiveTopologyMeta,
{
  type GPU = TypedMeshGPU<Self>;
  fn create(&self, device: &gpu::Device) -> Self::GPU {
    TypedMeshGPU {
      marker: Default::default(),
      inner: self.mesh.create_gpu(device),
    }
  }
  fn update(&self, gpu: &mut Self::GPU, device: &gpu::Device) {
    *gpu = self.create(device)
  }

  fn get_group(&self, group: MeshDrawGroup) -> MeshGroup {
    self.get_group(group)
  }

  fn draw(&self, group: MeshDrawGroup) -> DrawCommand {
    let range = self.get_group(group);
    DrawCommand::Indexed {
      base_vertex: 0,
      indices: range.into(),
      instances: 0..1,
    }
  }

  fn topology(&self) -> gpu::PrimitiveTopology {
    map_topology(T::ENUM)
  }

  fn build_shader(builder: &mut ShaderGraphRenderPipelineBuilder) {
    builder
      .vertex(|builder, _| {
        builder.register_vertex::<V>(VertexStepMode::Vertex);
        builder.primitive_state.topology = map_topology(T::ENUM);
        Ok(())
      })
      .unwrap();
  }
}

fn map_topology(pt: PrimitiveTopology) -> gpu::PrimitiveTopology {
  match pt {
    PrimitiveTopology::PointList => gpu::PrimitiveTopology::PointList,
    PrimitiveTopology::LineList => gpu::PrimitiveTopology::LineList,
    PrimitiveTopology::LineStrip => gpu::PrimitiveTopology::LineStrip,
    PrimitiveTopology::TriangleList => gpu::PrimitiveTopology::TriangleList,
    PrimitiveTopology::TriangleStrip => gpu::PrimitiveTopology::TriangleStrip,
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

impl<V, T, IU> IndexedMesh<T, Vec<V>, IU>
where
  V: Pod,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider,
  Self: GPUConsumableMeshBuffer,
{
  pub fn create_gpu(&self, device: &gpu::Device) -> MeshGPU {
    let vertex = bytemuck::cast_slice(self.vertex.as_slice());
    let vertex = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: vertex,
      usage: gpu::BufferUsages::VERTEX,
    });
    let vertex = vec![Rc::new(vertex)];

    let index = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: self.index.as_gpu_bytes(),
      usage: gpu::BufferUsages::INDEX,
    });
    let index = (Rc::new(index), self.index.format()).into();

    let range_full = MeshGroup {
      start: 0,
      count: self.draw_count(),
    };

    MeshGPU {
      vertex,
      index,
      range_full,
    }
  }
}
