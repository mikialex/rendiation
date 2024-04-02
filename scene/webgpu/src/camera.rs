use crate::*;

pub type CameraGPUGetter<'a> = &'a dyn Fn(&AllocIdx<SceneCameraImpl>) -> Option<CameraGPU>;

// todo, update not recreate
pub fn camera_gpus(
  projections: impl ReactiveCollection<AllocIdx<SceneCameraImpl>, Mat4<f32>>,
  node_mats: impl ReactiveCollection<NodeIdentity, Mat4<f32>>,
  camera_node_relations: impl ReactiveOneToManyRelationship<NodeIdentity, AllocIdx<SceneCameraImpl>>,
  cx: &ResourceGPUCtx,
) -> impl ReactiveCollection<AllocIdx<SceneCameraImpl>, CameraGPU> {
  let camera_world_mat = node_mats.one_to_many_fanout(camera_node_relations);

  camera_world_mat
    .collective_zip(projections)
    .collective_map(|(world, proj)| {
      let view = world.inverse_or_identity();
      let view_projection = proj * view;
      CameraGPUTransform {
        world,
        view,
        rotation: world.extract_rotation_mat(),

        projection: proj,
        projection_inv: proj.inverse_or_identity(),
        view_projection,
        view_projection_inv: view_projection.inverse_or_identity(),

        ..Zeroable::zeroed()
      }
    })
    .collective_create_uniforms(cx.clone())
    .collective_map(|ubo| CameraGPU { ubo })
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraGPU {
  pub ubo: UniformBufferCachedDataView<CameraGPUTransform>,
}

impl CameraGPU {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> BindingPreparer<ShaderUniformPtr<CameraGPUTransform>> {
    builder
      .bind_by(&self.ubo)
      .using_graphics_pair(builder, |r, camera| {
        let camera = camera.load().expand();
        r.register_typed_both_stage::<CameraViewMatrix>(camera.view);
        r.register_typed_both_stage::<CameraProjectionMatrix>(camera.projection);
        r.register_typed_both_stage::<CameraProjectionInverseMatrix>(camera.projection_inv);
        r.register_typed_both_stage::<CameraWorldMatrix>(camera.world);
        r.register_typed_both_stage::<CameraViewProjectionMatrix>(camera.view_projection);
        r.register_typed_both_stage::<CameraViewProjectionInverseMatrix>(
          camera.view_projection_inv,
        );
      })
  }

  pub fn new(device: &GPUDevice) -> Self {
    Self {
      ubo: create_uniform_with_cache(CameraGPUTransform::default(), device),
    }
  }
}

impl ShaderHashProvider for CameraGPU {}

impl ShaderPassBuilder for CameraGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo);
  }
}

impl GraphicsShaderProvider for CameraGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    let camera = self.inject_uniforms(builder);

    builder.vertex(|builder, _| {
      let camera = camera.using().load().expand();
      let position = builder.query::<WorldVertexPosition>()?;

      let mut clip_position = camera.view_projection * (position, val(1.)).into();

      let jitter = if let Ok(texel_size) = builder.query::<TexelSize>() {
        let jitter = texel_size * camera.jitter_normalized * clip_position.w();
        (jitter, val(0.), val(0.)).into()
      } else {
        Vec4::zero().into()
      };
      clip_position += jitter;

      builder.register::<ClipPosition>(clip_position);

      Ok(())
    })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct CameraGPUTransform {
  pub projection: Mat4<f32>,
  pub projection_inv: Mat4<f32>,

  pub rotation: Mat4<f32>,

  pub view: Mat4<f32>,
  pub world: Mat4<f32>,

  pub view_projection: Mat4<f32>,
  pub view_projection_inv: Mat4<f32>,

  /// jitter is always applied (cheap and reduce shader variance)
  /// range: -0.5 to 0.5
  pub jitter_normalized: Vec2<f32>,
}

pub fn setup_viewport(cb: &CameraViewBounds, pass: &mut GPURenderPass, buffer_size: Size) {
  let width: usize = buffer_size.width.into();
  let width = width as f32;
  let height: usize = buffer_size.height.into();
  let height = height as f32;
  pass.set_viewport(
    width * cb.to_left,
    height * cb.to_top,
    width * cb.width,
    height * cb.height,
    0.,
    1.,
  )
}
