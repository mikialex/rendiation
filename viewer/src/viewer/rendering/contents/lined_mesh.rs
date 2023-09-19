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
use webgpu::*;

use crate::*;

/// lined mesh is a way to draw on mesh edge line.
///
/// ## references
///
/// https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
/// https://tchayen.github.io/posts/wireframes-with-barycentric-coordinates
#[derive(Clone)]
pub struct LinedMesh {
  /// note, user should make sure the mesh not shared with others
  /// todo, impl runtime ownership checking
  mesh: SceneMeshType,
  #[allow(dead_code)] // todo
  lines: Vec<LineSegment<u32>>,
}

impl LinedMesh {
  pub fn new(mesh: SceneMeshType, lines: Vec<LineSegment<u32>>) -> Self {
    Self { mesh, lines }
  }
}
clone_self_incremental!(LinedMesh);

pub struct LinedMeshGPU {
  mesh_gpu: Box<MeshGPUInstance>,
  barycentric: GPUBufferResourceView,
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
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    let gpu = self.inner.as_ref();
    gpu.mesh_gpu.draw_command(group)
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

          let usage = webgpu::BufferUsages::VERTEX;
          let gpu = GPUBuffer::create(
            &ctx.gpu.device,
            BufferInit::WithInit(bytemuck::cast_slice(&buffer)),
            usage,
          );
          let gpu = GPUBufferResource::create_with_raw(gpu, usage).create_default_view();

          LinedMeshGPU {
            mesh_gpu: Box::new(mesh_gpu),
            barycentric: gpu,
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
  let mesh = mesh.read();
  let mesh = match &mesh.mesh {
    SceneMeshType::AttributesMesh(mesh) => mesh,
    _ => return None,
  };

  let _: AttributesMesh = mesh
    .read()
    .read_full()
    .primitive_iter()
    .filter_map(|p| match p {
      AttributeDynPrimitive::Triangle(t) => Some(t),
      _ => None,
    })
    .collect();

  // let barycentric = todo!

  //
  None
}

both!(BarycentricCoord, Vec3<f32>);

impl GraphicsShaderProvider for LinedMeshGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, _| {
      // todo, could we simplify this
      builder.push_single_vertex_layout::<BarycentricCoord>(VertexStepMode::Vertex);
      builder.set_vertex_out::<BarycentricCoord>(builder.query::<BarycentricCoord>().unwrap());
      Ok(())
    })
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| {
      let barycentric = builder.query::<BarycentricCoord>().unwrap();

      let line_color = val(Vec3::zero());
      let smoothing = val(1.);
      let thickness = val(1.);

      let deltas = barycentric.fwidth();
      let smoothing = deltas * smoothing;
      let thickness = deltas * thickness;
      let ratio = barycentric.smoothstep(thickness, thickness + smoothing);
      let ratio = ratio.x().min(ratio.y()).min(ratio.z());

      if let Ok(color) = builder.query::<ColorChannel>() {
        builder.register::<ColorChannel>(ratio.mix(line_color, color));
      }

      Ok(())
    })
  }
}

impl ShaderHashProvider for LinedMeshGPU {}
impl ShaderPassBuilder for LinedMeshGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh_gpu.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.barycentric)
  }
}

impl Stream for LinedMeshGPU {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}
