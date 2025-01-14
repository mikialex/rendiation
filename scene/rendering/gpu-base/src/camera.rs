use crate::*;

pub trait CameraRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>>;

  fn setup_camera_jitter(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
    jitter: Vec2<f32>,
    queue: &GPUQueue,
  );
}

pub type CameraUniforms =
  UniformUpdateContainer<EntityHandle<SceneCameraEntity>, CameraGPUTransform>;

pub fn camera_gpus(cx: &GPU) -> CameraUniforms {
  let source = camera_transforms()
    // todo, fix jitter override
    .collective_map(|t| CameraGPUTransform {
      world: t.world,
      view: t.view,
      rotation: t.rotation,

      projection: t.projection,
      projection_inv: t.projection_inv,
      view_projection: t.view_projection,
      view_projection_inv: t.view_projection_inv,

      ..Zeroable::zeroed()
    })
    .into_query_update_uniform(0, cx);

  CameraUniforms::default().with_source(source)
}

pub struct CameraGPU<'a> {
  pub ubo: &'a UniformBufferDataView<CameraGPUTransform>,
}

impl<'a> CameraGPU<'a> {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> BindingPreparer<ShaderUniformPtr<CameraGPUTransform>> {
    builder
      .bind_by_and_prepare(&self.ubo)
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
}

impl<'a> ShaderHashProvider for CameraGPU<'a> {
  shader_hash_type_id! {CameraGPU<'static>}
}

impl<'a> ShaderPassBuilder for CameraGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.ubo);
  }
}

impl<'a> GraphicsShaderProvider for CameraGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let camera = self.inject_uniforms(builder);

    builder.vertex(|builder, _| {
      let camera = camera.using().load().expand();
      if let Some(position) = builder.try_query::<WorldVertexPosition>() {
        let mut clip_position = camera.view_projection * (position, val(1.)).into();

        let jitter = if let Some(texel_size) = builder.try_query::<TexelSize>() {
          let jitter = texel_size * camera.jitter_normalized * clip_position.w();
          (jitter, val(0.), val(0.)).into()
        } else {
          Vec4::zero().into()
        };
        clip_position += jitter;

        builder.register::<ClipPosition>(clip_position);
      }
    })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq, UniformNodePtrAccess)]
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

#[derive(Default)]
pub struct DefaultGLESCameraRenderImplProvider {
  uniforms: UpdateResultToken,
}
pub struct DefaultGLESCameraRenderImpl {
  uniforms: LockReadGuardHolder<CameraUniforms>,
}

impl RenderImplProvider<Box<dyn CameraRenderImpl>> for DefaultGLESCameraRenderImplProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniforms = camera_gpus(cx);
    self.uniforms = source.register_multi_updater(uniforms);
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.uniforms);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn CameraRenderImpl> {
    Box::new(DefaultGLESCameraRenderImpl {
      uniforms: res.take_multi_updater_updated(self.uniforms).unwrap(),
    })
  }
}

impl CameraRenderImpl for DefaultGLESCameraRenderImpl {
  fn make_component(
    &self,
    idx: EntityHandle<SceneCameraEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let node = CameraGPU {
      ubo: self.uniforms.get(&idx)?,
    };
    Some(Box::new(node))
  }

  fn setup_camera_jitter(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
    jitter: Vec2<f32>,
    queue: &GPUQueue,
  ) {
    let uniform = self.uniforms.get(&camera).unwrap();
    uniform.write_at(
      queue,
      &jitter,
      offset_of!(CameraGPUTransform, jitter_normalized) as u64,
    );
  }
}
