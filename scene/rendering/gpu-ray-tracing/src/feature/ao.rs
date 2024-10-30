use crate::*;

pub struct SceneRayTracingAOFeature {
  desc: GPURaytracingPipelineDescriptor,
  // should we keep this?
  pipeline: Box<dyn GPURaytracingPipelineProvider>,
  sbt: Box<dyn ShaderBindingTableProvider>,
  scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlasInstance>,
  tex_io: RayTracingTextureIO,
}

#[derive(Clone)]
struct SceneRayTracingAOFeatureBinding {
  scene: TlasInstance,
  // camera: ,
}

impl ShaderHashProvider for SceneRayTracingAOFeatureBinding {
  shader_hash_type_id! {}
}

#[derive(Clone)]
struct SceneRayTracingAOFeatureInvocation {
  scene: Box<dyn GPUAccelerationStructureInvocationInstance>,
  // camera:
}

impl RayTracingCustomCtxProvider for SceneRayTracingAOFeatureBinding {
  type Invocation = SceneRayTracingAOFeatureInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    SceneRayTracingAOFeatureInvocation {
      scene: self.scene.create_invocation_instance(cx),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.scene.bind_pass(builder);
  }
}

struct RayTracingAOOutput;
impl RayTracingOutputTargetSemantic for RayTracingAOOutput {}

impl SceneRayTracingAOFeature {
  pub fn new(gpu: &GPU, tlas_size: Box<dyn Stream<Item = u32>>) -> Self {
    todo!()
  }

  pub fn render(
    &self,
    frame: &mut FrameCtx,
    system: Box<dyn GPURaytracingSystem>,
    previous_accumulation: GPU2DTextureView,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    self
      .tex_io
      .install_output_target::<RayTracingAOOutput>(previous_accumulation);

    let scene_source: SceneRayTracingAOFeatureBinding = todo!();

    let mut rtx_encoder = system.create_raytracing_encoder();

    rtx_encoder.set_pipeline(self.pipeline.as_ref());
    let canvas_size = frame.frame_size().into_u32();
    rtx_encoder.trace_ray((canvas_size.0, canvas_size.1, 1), self.sbt.as_ref());

    self.tex_io.take_output_target::<RayTracingAOOutput>();

    todo!()
  }
}
