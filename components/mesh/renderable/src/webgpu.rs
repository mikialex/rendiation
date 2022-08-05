use std::rc::Rc;

use bytemuck::Pod;
use core::marker::PhantomData;
use gpu::util::DeviceExt;
use gpu::DrawCommand;
use gpu::GPURenderPassCtx;
use rendiation_webgpu as gpu;
use shadergraph::*;

use crate::group::*;
use crate::mesh::*;

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

impl<I, V, T, IU> GPUMeshData for GroupedMesh<IndexedMesh<I, V, T, Vec<V>, IU>>
where
  V: Pod,
  V: ShaderGraphVertexInProvider,
  T: PrimitiveTopologyMeta<V>,
  I: gpu::IndexBufferSourceType,
  IU: AsRef<[u8]>,
  IndexedMesh<I, V, T, Vec<V>, IU>: AbstractMesh,
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
    match T::ENUM {
      PrimitiveTopology::PointList => gpu::PrimitiveTopology::PointList,
      PrimitiveTopology::LineList => gpu::PrimitiveTopology::LineList,
      PrimitiveTopology::LineStrip => gpu::PrimitiveTopology::LineStrip,
      PrimitiveTopology::TriangleList => gpu::PrimitiveTopology::TriangleList,
      PrimitiveTopology::TriangleStrip => gpu::PrimitiveTopology::TriangleStrip,
    }
  }

  fn build_shader(builder: &mut ShaderGraphRenderPipelineBuilder) {
    builder
      .vertex(|builder, _| {
        builder.register_vertex::<V>(VertexStepMode::Vertex);
        Ok(())
      })
      .unwrap();
  }
}

impl<I, V, T, IU> IndexedMesh<I, V, T, Vec<V>, IU>
where
  V: Pod,
  T: PrimitiveTopologyMeta<V>,
  I: gpu::IndexBufferSourceType,
  IU: AsRef<[u8]>,
  Self: AbstractMesh,
{
  pub fn create_gpu(&self, device: &gpu::Device) -> MeshGPU {
    let vertex = bytemuck::cast_slice(self.data.as_slice());
    let vertex = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: vertex,
      usage: gpu::BufferUsages::VERTEX,
    });
    let vertex = vec![Rc::new(vertex)];

    let index = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: self.index.as_ref(),
      usage: gpu::BufferUsages::INDEX,
    });
    let index = (Rc::new(index), I::FORMAT).into();

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
