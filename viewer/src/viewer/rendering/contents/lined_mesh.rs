use std::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::Stream;
use incremental::*;
use reactive::SignalStreamExt;
use rendiation_algebra::Vec3;
use rendiation_geometry::{LineSegment, OptionalNearest, Ray3};
use rendiation_shader_api::*;
use webgpu::{DrawCommand, GPUBuffer, GPURenderPassCtx, ShaderHashProvider, ShaderPassBuilder};

use crate::*;

/// lined mesh is a way to draw on mesh edge line.
#[derive(Clone)]
pub struct LinedMesh {
  /// note, user should make sure the mesh not shared with others
  /// todo, impl runtime ownership checking
  pub mesh: SceneMeshType,
  pub lines: Vec<LineSegment<u32>>,
}
clone_self_incremental!(LinedMesh);

pub struct LinedMeshGPU {
  mesh_gpu: Box<MeshGPUInstance>,
  barycentric: GPUBuffer,
}

/// line mesh not affect mesh shape
impl IntersectAbleGroupedMesh for LinedMesh {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    self.mesh.intersect_list_by_group(ray, conf, result, group)
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    self.mesh.intersect_nearest_by_group(ray, conf, group)
  }
}

type ReactiveLinedMeshGPUInner =
  impl AsRef<RenderComponentCell<LinedMeshGPU>> + Stream<Item = RenderComponentDeltaFlag>;

#[pin_project::pin_project]
pub struct ReactiveLinedMeshGPU {
  #[pin]
  inner: ReactiveLinedMeshGPUInner,
}

impl Stream for ReactiveLinedMeshGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

impl ReactiveRenderComponentSource for ReactiveLinedMeshGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl MeshDrawcallEmitter for ReactiveLinedMeshGPU {
  fn draw_command(&self, _group: MeshDrawGroup) -> DrawCommand {
    // self.inner.
    todo!()
  }
}

impl WebGPUMesh for LinedMesh {
  type ReactiveGPU = ReactiveLinedMeshGPU;

  fn create_reactive_gpu(
    source: &SharedIncrementalSignal<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let weak = source.downgrade();
    let ctx = ctx.clone();

    let create = move || {
      if let Some(m) = weak.upgrade() {
        if let Some((mesh, buffer)) = generate_barycentric_buffer_and_expanded_mesh(&m) {
          let mesh_gpu = mesh.create_scene_reactive_gpu(&ctx).unwrap();
          LinedMeshGPU {
            mesh_gpu: Box::new(mesh_gpu),
            barycentric: todo!(),
          }
          .into()
        } else {
          None
        }
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

    ReactiveLinedMeshGPU { inner }
  }
}

fn generate_barycentric_buffer_and_expanded_mesh(
  mesh: &SharedIncrementalSignal<LinedMesh>,
) -> Option<(SceneMeshType, Vec<Vec3<f32>>)> {
  None
}

impl GraphicsShaderProvider for LinedMeshGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, _| {
      // builder.register_vertex::<Vertex>(VertexStepMode::Vertex);
      // builder.register_vertex::<FatLineVertex>(VertexStepMode::Instance);
      // builder.primitive_state.topology = webgpu::PrimitiveTopology::TriangleList;
      // builder.primitive_state.cull_mode = None;
      Ok(())
    })
  }
}

impl ShaderHashProvider for LinedMeshGPU {}

impl ShaderPassBuilder for LinedMeshGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    // self.instance.setup_pass(ctx);
    // ctx.set_vertex_buffer_owned_next(&self.vertex);
  }
}

impl Stream for LinedMeshGPU {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}
