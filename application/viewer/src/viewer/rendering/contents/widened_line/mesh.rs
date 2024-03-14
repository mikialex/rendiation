use std::rc::Rc;

use __core::{
  pin::Pin,
  task::{Context, Poll},
};
use futures::Stream;
use incremental::*;
use reactive::*;
use rendiation_geometry::*;
use rendiation_mesh_core::{vertex::Vertex, *};
use rendiation_shader_api::*;
use webgpu::*;

use crate::*;

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

type ReactiveWidenedLineGPUImpl =
  impl AsRef<RenderComponentCell<WidenedLineMeshGPU>> + Stream<Item = RenderComponentDeltaFlag>;

#[pin_project::pin_project]
pub struct ReactiveWidenedLineGPU {
  #[pin]
  inner: ReactiveWidenedLineGPUImpl,
}

impl Stream for ReactiveWidenedLineGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

impl ReactiveRenderComponentSource for ReactiveWidenedLineGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl MeshDrawcallEmitter for ReactiveWidenedLineGPU {
  fn draw_command(&self, _group: MeshDrawGroup) -> DrawCommand {
    let range = self.inner.as_ref().inner.range_full;

    LINE_SEG_INSTANCE.with(|instance| DrawCommand::Indexed {
      base_vertex: 0,
      indices: 0..instance.draw_count() as u32,
      instances: range.into(),
    })
  }
}

impl WebGPUMesh for WidenedLineMesh {
  type ReactiveGPU = ReactiveWidenedLineGPU;

  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let weak = source.downgrade();
    let ctx = ctx.clone();

    let create = move || {
      if let Some(m) = weak.upgrade() {
        let mesh = m.read();
        let vertex = bytemuck::cast_slice(mesh.inner.mesh.data.as_slice());
        let vertex = create_gpu_buffer(vertex, webgpu::BufferUsages::VERTEX, &ctx.gpu.device)
          .create_default_view();

        let instance = ctx
          .custom_storage
          .write()
          .unwrap()
          .entry()
          .or_insert_with(|| create_widened_line_quad_gpu(&ctx.gpu.device))
          .data
          .clone();

        let range_full = MeshGroup {
          start: 0,
          count: mesh.inner.mesh.draw_count(),
        };

        Some(WidenedLineMeshGPU {
          vertex,
          instance,
          range_full,
        })
      } else {
        None
      }
    };

    let gpu = create().unwrap();
    let state = RenderComponentCell::new(gpu);

    let inner = source
      .single_listen_by::<()>(any_change_no_init)
      .fold_signal(state, move |_, state| {
        if let Some(gpu) = create() {
          state.inner = gpu;
          RenderComponentDeltaFlag::all().into()
        } else {
          None
        }
      });

    ReactiveWidenedLineGPU { inner }
  }
}

impl GraphicsShaderProvider for WidenedLineMeshGPU {
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

impl ShaderHashProvider for WidenedLineMeshGPU {}

impl ShaderPassBuilder for WidenedLineMeshGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.instance.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.vertex);
  }
}

pub struct WidenedLineMeshGPU {
  vertex: GPUBufferResourceView,
  /// All widened_line gpu instance shall share one instance buffer
  instance: Rc<MeshGPU>,
  range_full: MeshGroup,
}

impl Stream for WidenedLineMeshGPU {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
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

pub struct WidenedLineQuadInstance {
  data: Rc<MeshGPU>,
}

fn create_widened_line_quad() -> IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>> {
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
  IndexedMesh::new(data, index)
}

thread_local! {
  static LINE_SEG_INSTANCE: IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>> = create_widened_line_quad()
}

fn create_widened_line_quad_gpu(device: &webgpu::GPUDevice) -> WidenedLineQuadInstance {
  WidenedLineQuadInstance {
    data: Rc::new(LINE_SEG_INSTANCE.with(|f| create_gpu(f, device, Default::default()))),
  }
}
