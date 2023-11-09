use core::marker::PhantomData;

use bytemuck::Pod;
use rendiation_mesh_core::MeshGroupsInfo;
use rendiation_mesh_core::{GroupedMesh, IndexGet, MeshGroup};
use rendiation_shader_api::*;

use crate::*;

pub struct MeshGPU {
  range_full: MeshGroup,
  vertex: Vec<GPUBufferResourceView>,
  index: Option<(GPUBufferResourceView, IndexFormat)>,
  groups: MeshGroupsInfo,
}

pub struct TypedMeshGPU<T> {
  marker: PhantomData<T>,
  inner: MeshGPU,
}

impl<T> Stream for TypedMeshGPU<T> {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

impl<V, T, IU> GraphicsShaderProvider for TypedMeshGPU<GroupedMesh<IndexedMesh<T, Vec<V>, IU>>>
where
  V: ShaderVertexInProvider,
  T: PrimitiveTopologyMeta,
{
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, _| {
      builder.register_vertex::<V>(VertexStepMode::Vertex);
      builder.primitive_state.topology = map_topology(T::ENUM);
      Ok(())
    })
  }
}

impl<T> ShaderPassBuilder for TypedMeshGPU<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.setup_pass(ctx)
  }
}

/// variance info is encoded in T's type id
impl<T: 'static> ShaderHashProvider for TypedMeshGPU<T> {}

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
  fn format(&self) -> IndexFormat;
}

impl<T: IndexBufferSourceType> IndexBufferSourceTypeProvider for Vec<T> {
  fn format(&self) -> IndexFormat {
    T::FORMAT
  }
}

impl IndexBufferSourceTypeProvider for DynIndexContainer {
  fn format(&self) -> IndexFormat {
    match self {
      DynIndexContainer::Uint16(_) => u16::FORMAT,
      DynIndexContainer::Uint32(_) => u32::FORMAT,
    }
  }
}

impl<V, T, IU> ReactiveRenderComponentSource for ReactiveMeshGPUOfTypedMesh<V, T, IU>
where
  V: Pod + ShaderVertexInProvider + Unpin,
  T: PrimitiveTopologyMeta + Unpin,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider + Unpin + 'static,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
  GroupedMesh<IndexedMesh<T, Vec<V>, IU>>: IncrementalBase + Send + Sync,
{
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl<V, T, IU> MeshDrawcallEmitter for ReactiveMeshGPUOfTypedMesh<V, T, IU>
where
  V: Pod + ShaderVertexInProvider + Unpin,
  T: PrimitiveTopologyMeta + Unpin,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider + Unpin + 'static,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
  GroupedMesh<IndexedMesh<T, Vec<V>, IU>>: IncrementalBase + Send + Sync,
{
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    let inner: &TypedMeshGPU<GroupedMesh<IndexedMesh<T, Vec<V>, IU>>> = self.as_ref();

    let range = match group {
      MeshDrawGroup::Full => inner.inner.range_full,
      MeshDrawGroup::SubMesh(i) => inner.inner.groups.groups[i],
    };
    DrawCommand::Indexed {
      base_vertex: 0,
      indices: range.into(),
      instances: 0..1,
    }
  }
}

pub type ReactiveMeshGPUOfTypedMesh<V, T, IU>
where
  V: Pod + ShaderVertexInProvider + Unpin,
  T: PrimitiveTopologyMeta + Unpin,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider + Unpin + 'static,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
  GroupedMesh<IndexedMesh<T, Vec<V>, IU>>: IncrementalBase + Send + Sync,
= impl AsRef<RenderComponentCell<TypedMeshGPU<GroupedMesh<IndexedMesh<T, Vec<V>, IU>>>>>
  + Stream<Item = RenderComponentDeltaFlag>;

impl<V, T, IU> WebGPUMesh for GroupedMesh<IndexedMesh<T, Vec<V>, IU>>
where
  V: Pod + ShaderVertexInProvider + Unpin,
  T: PrimitiveTopologyMeta + Unpin,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider + Unpin + 'static,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
  Self: IncrementalBase + Send + Sync,
{
  type ReactiveGPU = ReactiveMeshGPUOfTypedMesh<V, T, IU>;

  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let ctx = ctx.clone();

    let create = move |m: &IncrementalSignalPtr<Self>| {
      let mesh = m.read();
      TypedMeshGPU {
        marker: Default::default(),
        inner: create_gpu(&mesh.mesh, &ctx.gpu.device, mesh.groups.clone()),
      }
    };

    let state = RenderComponentCell::new(create(source));

    source
      .single_listen_by::<()>(any_change_no_init)
      .filter_map_sync(source.defer_weak())
      .fold_signal(state, move |mesh, state| {
        state.inner = create(&mesh);
        RenderComponentDeltaFlag::all().into()
      })
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
  device: &GPUDevice,
  groups: MeshGroupsInfo,
) -> MeshGPU
where
  V: Pod,
  IU: IndexGet + AsGPUBytes + IndexBufferSourceTypeProvider,
  IndexedMesh<T, Vec<V>, IU>: GPUConsumableMeshBuffer,
{
  let vertex = bytemuck::cast_slice(mesh.vertex.as_slice());
  let vertex = create_gpu_buffer(vertex, BufferUsages::VERTEX, device).create_default_view();

  let vertex = vec![vertex];

  let index =
    create_gpu_buffer(mesh.index.as_gpu_bytes(), BufferUsages::INDEX, device).create_default_view();

  let index = (index, mesh.index.format()).into();

  let range_full = MeshGroup {
    start: 0,
    count: mesh.draw_count(),
  };

  MeshGPU {
    vertex,
    index,
    range_full,
    groups,
  }
}
