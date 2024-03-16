use incremental::*;
use reactive::*;
use rendiation_geometry::*;
use rendiation_mesh_core::{vertex::Vertex, *};
use rendiation_shader_api::*;
use webgpu::*;

use crate::*;

pub struct WidenLineMeshGPUResource {
  quad: IncrementalSignalPtr<AttributesMesh>,
  instance_buffers:
    Box<dyn ReactiveCollectionSelfContained<AllocIdx<WidenedLineMesh>, GPUBufferResourceView>>,
}

impl WidenLineMeshGPUResource {
  pub fn new(gpu: &ResourceGPUCtx) -> Self {
    let quad = create_widened_line_quad().into_ptr();

    let instance_buffers = storage_of::<WidenedLineMesh>()
      .listen_all_instance_changed_set()
      .collective_execute_gpu_map(gpu, |mesh, cx| {
        let vertex = bytemuck::cast_slice(mesh.inner.mesh.data.as_slice());
        create_gpu_buffer(vertex, webgpu::BufferUsages::VERTEX, &gpu.device).create_default_view()
      })
      .self_contain_into_boxed();

    Self {
      quad,
      instance_buffers,
    }
  }
}

#[derive(Clone)]
pub struct WidenedLineMesh {
  inner: GroupedMesh<NoneIndexedMesh<LineList, Vec<WidenedLineVertex>>>,
}
clone_self_incremental!(WidenedLineMesh);

impl WidenedLineMesh {
  pub fn new(inner: GroupedMesh<NoneIndexedMesh<LineList, Vec<WidenedLineVertex>>>) -> Self {
    Self { inner }
  }
}

impl IntersectAbleGroupedMesh for WidenedLineMesh {
  fn intersect_list_by_group(
    &self,
    _ray: Ray3,
    _conf: &MeshBufferIntersectConfig,
    _result: &mut MeshBufferHitList,
    _group: MeshDrawGroup,
  ) {
  }

  fn intersect_nearest_by_group(
    &self,
    _ray: Ray3,
    _conf: &MeshBufferIntersectConfig,
    _group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    OptionalNearest::none()
  }
}

pub struct WidenedLineMeshGPU<'a> {
  inner: AttributesMeshGPU<'a>,
  vertex: &'a GPUBufferResourceView,
  origin: &'a WidenedLineMesh,
}

impl<'a> GraphicsShaderProvider for WidenedLineMeshGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, _| {
      builder.register_vertex::<Vertex>(VertexStepMode::Vertex);
      builder.register_vertex::<WidenedLineVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
      Ok(())
    })
  }
}

impl<'a> ShaderHashProvider for WidenedLineMeshGPU<'a> {}

impl<'a> ShaderPassBuilder for WidenedLineMeshGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.instance.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.vertex);
  }
}

impl<'a> MeshDrawcallEmitter for WidenedLineMeshGPU<'a> {
  fn draw_command(&self, _group: MeshDrawGroup) -> DrawCommand {
    // let range = self.inner.as_ref().inner.range_full;

    // LINE_SEG_INSTANCE.with(|instance| DrawCommand::Indexed {
    //   base_vertex: 0,
    //   indices: 0..instance.draw_count() as u32,
    //   instances: range.into(),
    // })
    todo!()
  }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct WidenedLineVertex {
  #[semantic(WidenedLineStart)]
  pub start: Vec3<f32>,
  #[semantic(WidenedLineEnd)]
  pub end: Vec3<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}

only_vertex!(WidenedLineStart, Vec3<f32>);
only_vertex!(WidenedLineEnd, Vec3<f32>);

fn create_widened_line_quad() -> AttributesMesh {
  #[rustfmt::skip]
  let positions: Vec<isize> = vec![- 1, 2, 0, 1, 2, 0, - 1, 1, 0, 1, 1, 0, - 1, 0, 0, 1, 0, 0, - 1, - 1, 0, 1, - 1, 0];
  let positions: &[Vec3<isize>] = bytemuck::cast_slice(positions.as_slice());
  let uvs: Vec<isize> = vec![-1, 2, 1, 2, -1, 1, 1, 1, -1, -1, 1, -1, -1, -2, 1, -2];
  let uvs: &[Vec2<isize>] = bytemuck::cast_slice(uvs.as_slice());

  let data: Vec<_> = positions
    .iter()
    .zip(uvs)
    .map(|(position, uv)| Vertex {
      position: position.map(|v| v as f32),
      normal: Vec3::new(0., 0., 1.),
      uv: uv.map(|v| v as f32),
    })
    .collect();

  let index = vec![0, 2, 1, 2, 3, 1, 2, 4, 3, 4, 5, 3, 4, 6, 5, 6, 7, 5];
  IndexedMesh::new(data, index);

  AttributeMeshData {
    attributes: todo!(),
    indices: todo!(),
    mode: todo!(),
    groups: todo!(),
  }
  .build()
}
