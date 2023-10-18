use std::{
  pin::Pin,
  task::{Context, Poll},
};

use __core::hash::{Hash, Hasher};
use futures::Stream;
use incremental::*;
use reactive::SignalStreamExt;
use rendiation_algebra::Vec3;
use rendiation_geometry::{OptionalNearest, Ray3, Triangle};
use rendiation_shader_api::*;
use webgpu::*;

use crate::*;

/// solid lined mesh is a way to draw on mesh edge line.
///
/// ## references
///
/// https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
/// https://tchayen.github.io/posts/wireframes-with-barycentric-coordinates
#[derive(Clone)]
pub struct SolidLinedMesh {
  /// note, user should make sure the mesh not shared with others
  /// todo, impl runtime ownership checking
  mesh: MeshEnum,
}

impl SolidLinedMesh {
  pub fn new(mesh: MeshEnum) -> Self {
    Self { mesh }
  }
}
clone_self_incremental!(SolidLinedMesh);

/// line mesh not affect mesh shape
impl IntersectAbleGroupedMesh for SolidLinedMesh {
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

/// we do need a new gpu type to inject the mesh line shading logic, not only the resource part
pub struct SolidLinedMeshGPU {
  mesh_gpu: MeshGPUInstance,
}

type ReactiveSolidLinedMeshGPUImpl =
  impl AsRef<RenderComponentCell<SolidLinedMeshGPU>> + Stream<Item = RenderComponentDeltaFlag>;

#[pin_project::pin_project]
pub struct ReactiveSolidLinedMeshGPU {
  #[pin]
  inner: ReactiveSolidLinedMeshGPUImpl,
}

impl Stream for ReactiveSolidLinedMeshGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

impl ReactiveRenderComponentSource for ReactiveSolidLinedMeshGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl MeshDrawcallEmitter for ReactiveSolidLinedMeshGPU {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    let gpu = self.inner.as_ref();
    gpu.mesh_gpu.draw_command(group)
  }
}

impl WebGPUMesh for SolidLinedMesh {
  type ReactiveGPU = ReactiveSolidLinedMeshGPU;

  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let weak = source.downgrade();
    let ctx = ctx.clone();

    let create = move || {
      if let Some(m) = weak.upgrade() {
        if let Some(mesh) = generate_barycentric_buffer_and_expanded_mesh(&m) {
          let mesh_gpu = mesh.create_scene_reactive_gpu(&ctx).unwrap();

          SolidLinedMeshGPU { mesh_gpu }.into()
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

    ReactiveSolidLinedMeshGPU { inner }
  }
}

#[derive(Clone, Copy)]
struct FullReaderReadWithBarycentric<'a> {
  inner: FullReaderRead<'a>,
  barycentric: Vec3<f32>,
}

impl<'a> Eq for FullReaderReadWithBarycentric<'a> {}
impl<'a> PartialEq for FullReaderReadWithBarycentric<'a> {
  fn eq(&self, other: &Self) -> bool {
    self.inner == other.inner && self.barycentric == other.barycentric
  }
}

impl<'a> Hash for FullReaderReadWithBarycentric<'a> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inner.hash(state);
    self.barycentric.map(|f| f.to_bits()).hash(state);
  }
}

impl<'a> AttributeVertex for FullReaderReadWithBarycentric<'a> {
  fn layout(&self) -> Vec<AttributeSemantic> {
    let mut inner = self.inner.layout();

    get_dyn_trait_downcaster_static!(CustomAttributeKeyGPU)
      .register::<BarycentricCoordAttributeKey>();

    inner.push(AttributeSemantic::Foreign(ForeignAttributeKey::new(
      BarycentricCoordAttributeKey,
    )));
    inner
  }

  fn write(self, target: &mut [Vec<u8>]) {
    self
      .inner
      .write(target.get_mut(0..target.len() - 1).unwrap());
    target
      .last_mut()
      .unwrap()
      .extend_from_slice(bytemuck::bytes_of(&self.barycentric))
  }
}

fn generate_barycentric_buffer_and_expanded_mesh(
  mesh: &IncrementalSignalPtr<SolidLinedMesh>,
) -> Option<MeshEnum> {
  let mesh = mesh.read();
  let mesh = match &mesh.mesh {
    MeshEnum::AttributesMesh(mesh) => mesh,
    _ => return None,
  };

  let mesh: AttributeMeshData = mesh
    .read()
    .read_full()
    .primitive_iter()
    .filter_map(|p| match p {
      AttributeDynPrimitive::Triangle(t) => Some(t),
      _ => None,
    })
    .map(|tri| {
      Triangle::new(
        FullReaderReadWithBarycentric {
          inner: tri.a,
          barycentric: Vec3::new(1., 0., 0.),
        },
        FullReaderReadWithBarycentric {
          inner: tri.b,
          barycentric: Vec3::new(0., 1., 0.),
        },
        FullReaderReadWithBarycentric {
          inner: tri.c,
          barycentric: Vec3::new(0., 0., 1.),
        },
      )
    })
    .collect();

  let mesh = mesh.build();

  MeshEnum::AttributesMesh(mesh.into_ptr()).into()
}

#[derive(Clone, Copy)]
struct BarycentricCoordAttributeKey;

type_as_dyn_trait!(BarycentricCoordAttributeKey, AttributeReadSchema);
impl AttributeReadSchema for BarycentricCoordAttributeKey {
  fn item_byte_size(&self) -> usize {
    3 * 4
  }
}

type_as_dyn_trait!(BarycentricCoordAttributeKey, CustomAttributeKeyGPU);
impl CustomAttributeKeyGPU for BarycentricCoordAttributeKey {
  fn inject_shader(&self, builder: &mut ShaderVertexBuilder) {
    builder.push_single_vertex_layout::<BarycentricCoord>(VertexStepMode::Vertex);
  }
}

both!(BarycentricCoord, Vec3<f32>);

impl GraphicsShaderProvider for SolidLinedMeshGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.mesh_gpu.build(builder)?;
    builder.vertex(|builder, _| {
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
      // builder.register::<ColorChannel>(barycentric);

      Ok(())
    })
  }
}

impl ShaderHashProvider for SolidLinedMeshGPU {}
impl ShaderPassBuilder for SolidLinedMeshGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh_gpu.setup_pass(ctx);
  }
}

impl Stream for SolidLinedMeshGPU {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}
