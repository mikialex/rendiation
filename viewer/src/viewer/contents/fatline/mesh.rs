use std::rc::Rc;

use __core::{
  pin::Pin,
  task::{Context, Poll},
};
use futures::Stream;
use incremental::*;
use reactive::*;
use rendiation_geometry::*;
use rendiation_renderable_mesh::{vertex::Vertex, *};
use shadergraph::*;
use webgpu::*;

use crate::*;

#[derive(Clone)]
pub struct FatlineMesh {
  inner: GroupedMesh<NoneIndexedMesh<LineList, Vec<FatLineVertex>>>,
}
clone_self_incremental!(FatlineMesh);

impl FatlineMesh {
  pub fn new(inner: GroupedMesh<NoneIndexedMesh<LineList, Vec<FatLineVertex>>>) -> Self {
    Self { inner }
  }
}

impl IntersectAbleGroupedMesh for FatlineMesh {
  fn intersect_list(
    &self,
    _ray: Ray3,
    _conf: &MeshBufferIntersectConfig,
    _result: &mut MeshBufferHitList,
    _group: MeshDrawGroup,
  ) {
  }

  fn intersect_nearest(
    &self,
    _ray: Ray3,
    _conf: &MeshBufferIntersectConfig,
    _group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    OptionalNearest::none()
  }
}

type ReactiveFatlineGPUInner =
  impl AsRef<RenderComponentCell<FatlineMeshGPU>> + Stream<Item = RenderComponentDeltaFlag>;

#[pin_project::pin_project]
pub struct ReactiveFatlineGPU {
  #[pin]
  inner: ReactiveFatlineGPUInner,
}

impl Stream for ReactiveFatlineGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

impl ReactiveRenderComponentSource for ReactiveFatlineGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl WebGPUMesh for FatlineMesh {
  type ReactiveGPU = ReactiveFatlineGPU;

  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
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
          .or_insert_with(|| create_fatline_quad_gpu(&ctx.gpu.device))
          .data
          .clone();

        Some(FatlineMeshGPU { vertex, instance })
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

    ReactiveFatlineGPU { inner }
  }

  fn draw_impl<'a>(&self, group: MeshDrawGroup) -> DrawCommand {
    FATLINE_INSTANCE.with(|instance| DrawCommand::Indexed {
      base_vertex: 0,
      indices: 0..instance.draw_count() as u32,
      instances: self.inner.get_group(group).into(),
    })
  }
}

impl ShaderGraphProvider for FatlineMeshGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      builder.register_vertex::<Vertex>(VertexStepMode::Vertex);
      builder.register_vertex::<FatLineVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
      Ok(())
    })
  }
}

impl ShaderHashProvider for FatlineMeshGPU {}

impl ShaderPassBuilder for FatlineMeshGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.instance.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.vertex);
  }
}

pub struct FatlineMeshGPU {
  vertex: GPUBufferResourceView,
  /// All fatline gpu instance shall share one instance buffer
  instance: Rc<MeshGPU>,
}

impl Stream for FatlineMeshGPU {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct FatLineVertex {
  #[semantic(FatLineStart)]
  pub start: Vec3<f32>,
  #[semantic(FatLineEnd)]
  pub end: Vec3<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}

only_vertex!(FatLineStart, Vec3<f32>);
only_vertex!(FatLineEnd, Vec3<f32>);

pub struct FatlineQuadInstance {
  data: Rc<MeshGPU>,
}

fn create_fatline_quad() -> IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>> {
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
  static FATLINE_INSTANCE: IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>> = create_fatline_quad()
}

fn create_fatline_quad_gpu(device: &webgpu::GPUDevice) -> FatlineQuadInstance {
  FatlineQuadInstance {
    data: Rc::new(FATLINE_INSTANCE.with(|f| create_gpu(f, device))),
  }
}
