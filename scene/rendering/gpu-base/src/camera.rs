use crate::*;

pub fn use_camera_uniforms(
  cx: &mut impl QueryGPUHookCx,
  camera_source: &RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
) -> Option<CameraRenderer> {
  cx.use_uniform_buffers::<EntityHandle<SceneCameraEntity>, CameraGPUTransform>(|source, cx| {
    source.with_source(
      camera_source
        .clone()
        // todo, fix jitter override
        .collective_map(CameraGPUTransform::from)
        .into_query_update_uniform(0, cx),
    )
  })
  .map(CameraRenderer)
}

pub type CameraUniforms =
  UniformUpdateContainer<EntityHandle<SceneCameraEntity>, CameraGPUTransform>;

pub fn camera_gpus(
  cx: &GPU,
  camera_transforms: impl ReactiveQuery<Key = EntityHandle<SceneCameraEntity>, Value = CameraTransform>,
) -> CameraUniforms {
  let source = camera_transforms
    // todo, fix jitter override
    .collective_map(CameraGPUTransform::from)
    .into_query_update_uniform(0, cx);

  CameraUniforms::default().with_source(source)
}

#[derive(Clone)]
pub struct CameraGPU {
  pub ubo: UniformBufferDataView<CameraGPUTransform>,
}

only_vertex!(CameraJitter, Vec2<f32>);

impl CameraGPU {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> GraphicsPairInputNodeAccessor<UniformBufferDataView<CameraGPUTransform>> {
    builder
      .bind_by_and_prepare(&self.ubo)
      .using_graphics_pair(|r, camera| {
        let camera = camera.load().expand();

        r.register_typed_both_stage::<CameraWorldPositionHP>(hpt_uniform_to_hpt(
          camera.world_position,
        ));
        r.register_typed_both_stage::<CameraViewNoneTranslationMatrix>(
          camera.view_projection_inv_without_translation,
        );
        r.register_typed_both_stage::<CameraProjectionMatrix>(camera.projection);
        r.register_typed_both_stage::<CameraProjectionInverseMatrix>(camera.projection_inv);
        r.register_typed_both_stage::<CameraWorldNoneTranslationMatrix>(
          camera.world_without_translation,
        );
        r.register_typed_both_stage::<CameraViewNoneTranslationProjectionMatrix>(
          camera.view_projection_without_translation,
        );
        r.register_typed_both_stage::<CameraViewNoneTranslationProjectionInverseMatrix>(
          camera.view_projection_inv_without_translation,
        );
        r.register_vertex_stage::<CameraJitter>(camera.jitter_normalized);
      })
  }
}

impl ShaderHashProvider for CameraGPU {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for CameraGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo);
  }
}

impl GraphicsShaderProvider for CameraGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.inject_uniforms(builder);

    builder.vertex(|builder, _| {
      // this check enables self as a uniform inject only render component without dependency on node.
      if builder.try_query::<WorldNoneTranslationMatrix>().is_some() {
        let local_position = builder.query::<GeometryPosition>();
        let object_world_position = builder.query::<WorldPositionHP>();
        let (clip_position, render_position) =
          camera_transform_impl(builder, local_position, object_world_position);

        builder.register::<RenderVertexPosition>(render_position);
        builder.register::<ClipPosition>(clip_position);
      }
    })
  }
}

/// return (clip space position, render space position)
pub fn camera_transform_impl(
  builder: &mut ShaderVertexBuilder,
  position_in_local_space: Node<Vec3<f32>>,
  object_world_position: Node<HighPrecisionTranslation>,
) -> (Node<Vec4<f32>>, Node<Vec3<f32>>) {
  let world_mat_no_translation = builder.query::<WorldNoneTranslationMatrix>();
  let camera_world_position = builder.query::<CameraWorldPositionHP>();
  let world_to_render_offset = hpt_sub_hpt(object_world_position, camera_world_position);
  let translate_into_render_space: Node<Vec4<f32>> = (world_to_render_offset, val(0.)).into();

  let world_transformed_without_translation =
    world_mat_no_translation * (position_in_local_space, val(1.)).into();
  let position_in_render_space =
    world_transformed_without_translation + translate_into_render_space;

  let view_projection_none_translation =
    builder.query::<CameraViewNoneTranslationProjectionMatrix>();

  let mut clip_position = view_projection_none_translation * position_in_render_space;

  let jitter = if let Some(texel_size) = builder.try_query::<TexelSize>() {
    let jitter = builder.query::<CameraJitter>();
    let jitter = texel_size * jitter * clip_position.w();
    (jitter, val(0.), val(0.)).into()
  } else {
    Vec4::zero().into()
  };
  clip_position += jitter;

  (clip_position, position_in_render_space.xyz())
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct CameraGPUTransform {
  pub projection: Mat4<f32>,
  pub projection_inv: Mat4<f32>,

  pub view_without_translation: Mat4<f32>,
  pub world_without_translation: Mat4<f32>,
  pub world_position: HighPrecisionTranslationUniform,

  pub view_projection_without_translation: Mat4<f32>,
  pub view_projection_inv_without_translation: Mat4<f32>,

  /// contains low precision translation, currently should only used in ray tracing
  pub view_projection_inv: Mat4<f32>,

  /// jitter is always applied (cheap and reduce shader variance)
  /// range: -0.5 to 0.5
  pub jitter_normalized: Vec2<f32>,
}

impl From<CameraTransform> for CameraGPUTransform {
  fn from(t: CameraTransform) -> Self {
    let (world_without_translation, world_position) = into_mat_hpt_uniform_pair(t.world);
    let (view_without_translation, _) = into_mat_hpt_uniform_pair(t.view);

    let view_projection_without_translation = t.projection * t.view.into_f32().remove_position();

    Self {
      world_without_translation,
      world_position,
      view_without_translation,

      projection: t.projection,
      projection_inv: t.projection_inv,
      view_projection_without_translation,
      view_projection_inv_without_translation: view_projection_without_translation
        .inverse_or_identity(),
      view_projection_inv: t.view_projection_inv.into_f32(),
      ..Zeroable::zeroed()
    }
  }
}

// pub fn setup_viewport(cb: &CameraViewBounds, pass: &mut GPURenderPass, buffer_size: Size) {
//   let width: usize = buffer_size.width.into();
//   let width = width as f32;
//   let height: usize = buffer_size.height.into();
//   let height = height as f32;
//   pass.set_viewport(
//     width * cb.to_left,
//     height * cb.to_top,
//     width * cb.width,
//     height * cb.height,
//     0.,
//     1.,
//   )
// }

#[derive(Clone)]
pub struct CameraRenderer(pub LockReadGuardHolder<CameraUniforms>);

impl CameraRenderer {
  pub fn make_component(&self, idx: EntityHandle<SceneCameraEntity>) -> Option<CameraGPU> {
    CameraGPU {
      ubo: self.0.get(&idx)?.clone(),
    }
    .into()
  }

  pub fn setup_camera_jitter(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
    jitter: Vec2<f32>,
    queue: &GPUQueue,
  ) {
    let uniform = self.0.get(&camera).unwrap();
    uniform.write_at(
      queue,
      &jitter,
      offset_of!(CameraGPUTransform, jitter_normalized) as u64,
    );
  }
}
