use crate::*;

mod egui;
mod frame_logic;
mod grid_ground;
mod lighting;
mod outline;
mod ray_tracing;
mod widget;

mod g_buffer;
pub use g_buffer::*;
pub use ray_tracing::*;

mod post;
pub use frame_logic::*;
use futures::Future;
use grid_ground::*;
pub use lighting::*;
pub use post::*;
use reactive::EventSource;
use rendiation_device_ray_tracing::GPUWaveFrontComputeRaytracingSystem;
use rendiation_occlusion_culling::GPUTwoPassOcclusionCulling;
use rendiation_scene_rendering_gpu_indirect::build_default_indirect_render_system;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_texture_gpu_process::copy_frame;
use rendiation_webgpu::*;
use widget::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RasterizationRenderBackendType {
  Gles,
  Indirect,
}

pub type BoxedSceneRenderImplProvider =
  BoxedQueryBasedGPUFeature<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>>;

#[derive(Clone, Copy)]
pub struct ViewerNDC {
  pub enable_reverse_z: bool,
}

/// currently, the reverse z is implement by a custom ndc space mapper.
/// this is conceptually wrong because ndc is not changed at all.
/// however it's convenient to do so because the reverse operation must implement in projection(not post transform)
/// and ndc space mapper create a good place to inject projection modification logic.
impl<T: Scalar> NDCSpaceMapper<T> for ViewerNDC {
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T> {
    let mut m = WebGPUxNDC.transform_from_opengl_standard_ndc();

    if self.enable_reverse_z {
      m.c3 = -T::half()
    }
    m
  }
}

pub fn init_renderer(
  updater: &mut ReactiveQueryCtx,
  ty: RasterizationRenderBackendType,
  gpu: &GPU,
  camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  enable_reverse_z: bool,
) -> BoxedSceneRenderImplProvider {
  let prefer_bindless_textures = false;
  let mut renderer_impl = match ty {
    RasterizationRenderBackendType::Gles => {
      log::info!("init gles rendering");
      Box::new(build_default_gles_render_system(
        gpu,
        prefer_bindless_textures,
        camera_source,
        enable_reverse_z,
        std::sync::Arc::new(|_, _| {}),
      )) as BoxedSceneRenderImplProvider
    }
    RasterizationRenderBackendType::Indirect => {
      log::info!("init indirect rendering");
      Box::new(build_default_indirect_render_system(
        gpu,
        prefer_bindless_textures,
        camera_source,
        enable_reverse_z,
      ))
    }
  };

  renderer_impl.register(updater, gpu);
  renderer_impl
}

pub struct Viewer3dRenderingCtx {
  frame_index: u64,
  ndc: ViewerNDC,
  frame_logic: ViewerFrameLogic,
  rendering_resource: ReactiveQueryCtx,
  renderer_impl: BoxedSceneRenderImplProvider,
  indirect_occlusion_culling_impl: Option<GPUTwoPassOcclusionCulling>,
  current_renderer_impl_ty: RasterizationRenderBackendType,
  rtx_renderer_impl: Option<RayTracingSystemGroup>,
  rtx_effect_mode: RayTracingEffectMode,
  rtx_rendering_enabled: bool,
  lighting: LightSystem,
  pool: AttachmentPool,
  gpu: GPU,
  swap_chain: ApplicationWindowSurface,
  on_encoding_finished: EventSource<ViewRenderedState>,
  expect_read_back_for_next_render_result: bool,
  camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub(crate) picker: GPUxEntityIdMapPicker,
  pub(crate) statistics: FramePassStatistics,
  pub enable_statistic_collect: bool,

  stat_frame_time_in_ms: StatisticStore<f32>,
  last_render_timestamp: Option<Instant>,
}

impl Viewer3dRenderingCtx {
  pub fn gpu(&self) -> &GPU {
    &self.gpu
  }

  pub fn tick_frame(&mut self) {
    self.pool.tick();
  }

  pub fn new(
    gpu: GPU,
    swap_chain: ApplicationWindowSurface,
    ndc: ViewerNDC,
    camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  ) -> Self {
    let mut rendering_resource = ReactiveQueryCtx::default();

    let lighting =
      LightSystem::new_and_register(&mut rendering_resource, &gpu, ndc.enable_reverse_z, ndc);

    let camera_source_init = camera_source.clone_as_static();

    let renderer_impl = init_renderer(
      &mut rendering_resource,
      RasterizationRenderBackendType::Gles,
      &gpu,
      camera_source,
      ndc.enable_reverse_z,
    );

    Self {
      enable_statistic_collect: false,
      frame_index: 0,
      ndc,
      swap_chain,
      indirect_occlusion_culling_impl: None,
      rendering_resource,
      renderer_impl,
      current_renderer_impl_ty: RasterizationRenderBackendType::Gles,
      rtx_renderer_impl: None, // late init
      rtx_effect_mode: RayTracingEffectMode::ReferenceTracing,
      rtx_rendering_enabled: false,
      frame_logic: ViewerFrameLogic::new(&gpu),
      lighting,
      pool: init_attachment_pool(&gpu),
      statistics: FramePassStatistics::new(64, &gpu),
      gpu,
      on_encoding_finished: Default::default(),
      expect_read_back_for_next_render_result: false,
      camera_source: camera_source_init,
      picker: Default::default(),
      stat_frame_time_in_ms: StatisticStore::new(200),
      last_render_timestamp: Default::default(),
    }
  }

  pub fn set_enable_indirect_occlusion_culling_support(&mut self, enable: bool) {
    if enable {
      if self.indirect_occlusion_culling_impl.is_none() {
        self.indirect_occlusion_culling_impl =
          GPUTwoPassOcclusionCulling::new(u16::MAX as usize).into();
      }
    } else {
      self.indirect_occlusion_culling_impl = None
    }
  }

  pub fn set_enable_rtx_rendering_support(&mut self, enable: bool) {
    if enable {
      if self.rtx_renderer_impl.is_none() {
        let rtx_backend_system = GPUWaveFrontComputeRaytracingSystem::new(&self.gpu);
        let rtx_system = RtxSystemCore::new(Box::new(rtx_backend_system));
        let mut rtx_renderer_impl =
          RayTracingSystemGroup::new(&rtx_system, &self.gpu, self.camera_source.clone_as_static());

        rtx_renderer_impl.register(&mut self.rendering_resource, &self.gpu);

        self.rtx_renderer_impl = Some(rtx_renderer_impl);
      }
    } else {
      if let Some(rtx) = &mut self.rtx_renderer_impl {
        rtx.deregister(&mut self.rendering_resource);
      }
      self.rtx_renderer_impl = None;
    }
  }

  /// only texture could be read. caller must sure the target passed in render call not using
  /// window surface.
  #[allow(unused)] // used in terminal command
  pub fn read_next_render_result(
    &mut self,
  ) -> impl Future<Output = Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr>> {
    self.expect_read_back_for_next_render_result = true;
    use futures::FutureExt;
    self
      .on_encoding_finished
      .once_future(|result| result.clone().read())
      .flatten()
  }

  #[instrument(name = "frame rendering", skip_all)]
  pub fn render(
    &mut self,
    target: &RenderTargetView,
    content: &Viewer3dSceneCtx,
    scene_derive: &Viewer3dSceneDerive,
    cx: &mut Context,
  ) {
    self.frame_index += 1;
    let now = Instant::now();
    if let Some(last_frame_time) = self.last_render_timestamp.take() {
      self.stat_frame_time_in_ms.insert(
        now.duration_since(last_frame_time).as_secs_f32() * 1000.,
        self.frame_index,
      );
    }
    self.last_render_timestamp = Some(now);

    let span = span!(Level::INFO, "update all rendering resource");
    let mut resource = self.rendering_resource.poll_update_all(cx);
    drop(span);

    let renderer = self.renderer_impl.create_impl(&mut resource);

    let statistics = self
      .enable_statistic_collect
      .then(|| self.statistics.create_resolver(self.frame_index));

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool, statistics);

    let render_target = if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      target.create_attachment_key().request(&ctx)
    } else {
      target.clone()
    };

    if self.rtx_rendering_enabled {
      if let Some(rtx_renderer_impl) = &mut self.rtx_renderer_impl {
        let mut rtx_renderer = rtx_renderer_impl.create_impl(&mut resource);

        match self.rtx_effect_mode {
          RayTracingEffectMode::AO => {
            if rtx_renderer.base.any_changed {
              rtx_renderer.ao.reset_sample();
            }

            let ao_result = rtx_renderer.ao.render(
              &mut ctx,
              &mut rtx_renderer.base,
              content.scene,
              content.main_camera,
            );

            pass("copy rtx ao into final target")
              .with_color(target, store_full_frame())
              .render_ctx(&mut ctx)
              .by(&mut copy_frame(RenderTargetView::from(ao_result), None));
          }
          RayTracingEffectMode::ReferenceTracing => {
            if rtx_renderer.base.any_changed {
              rtx_renderer.pt.reset_sample();
            }

            let result = rtx_renderer.pt.render(
              &mut ctx,
              &mut rtx_renderer.base,
              content.scene,
              content.main_camera,
              &self.lighting.tonemap,
            );
            pass("copy pt result into final target")
              .with_color(target, store_full_frame())
              .render_ctx(&mut ctx)
              .by(&mut copy_frame(RenderTargetView::from(result), None));
          }
        }

        self.picker.notify_frame_id_buffer_not_available();
      }
    } else {
      let lighting_cx = self.lighting.prepare_and_create_impl(
        &mut resource,
        &mut ctx,
        cx,
        renderer.as_ref(),
        content.scene,
      );

      let current_view_projection_inv = scene_derive
        .camera_transforms
        .access(&content.main_camera)
        .unwrap()
        .view_projection_inv;

      let entity_id = self.frame_logic.render(
        &mut ctx,
        renderer.as_ref(),
        scene_derive,
        &lighting_cx,
        content,
        &render_target,
        current_view_projection_inv,
        self.ndc.enable_reverse_z,
      );

      let entity_id = entity_id.expect_standalone_common_texture_view();
      self.picker.read_new_frame_id_buffer(
        &GPUTypedTextureView::<TextureDimension2, u32>::try_from(entity_id.clone()).unwrap(),
        &self.gpu,
        &mut ctx.encoder,
      );
      //
    }

    {
      let widgets_result = draw_widgets(
        &mut ctx,
        renderer.as_ref(),
        content.widget_scene,
        self.ndc.enable_reverse_z,
        content.main_camera,
        &self.frame_logic.axis,
      );
      let mut copy_scene_msaa_widgets = copy_frame(
        widgets_result,
        BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
      );
      pass("copy_scene_msaa_widgets")
        .with_color(&render_target, load_and_store())
        .render_ctx(&mut ctx)
        .by(&mut copy_scene_msaa_widgets);
    }

    // do extra copy to surface texture
    if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      pass("extra final copy to surface")
        .with_color(target, store_full_frame())
        .render_ctx(&mut ctx)
        .by(&mut rendiation_texture_gpu_process::copy_frame(
          render_target.clone(),
          None,
        ));
    }
    self.expect_read_back_for_next_render_result = false;
    drop(ctx);

    noop_ctx!(cx);
    self.statistics.poll(cx);

    self.on_encoding_finished.emit(&ViewRenderedState {
      target: render_target,
      device: self.gpu.device.clone(),
      queue: self.gpu.queue.clone(),
    });
  }
}

#[derive(Clone)]
struct ViewRenderedState {
  target: RenderTargetView,
  device: GPUDevice,
  queue: GPUQueue,
}

#[derive(Debug)]
pub enum ViewerRenderResultReadBackErr {
  Gpu(rendiation_webgpu::BufferAsyncError),
  UnableToReadSurfaceTexture,
}

impl ViewRenderedState {
  async fn read(self) -> Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr> {
    let tex = match self.target {
      RenderTargetView::Texture(tex) => tex.clone(),
      RenderTargetView::ReusedTexture(tex) => tex.item().clone(),
      RenderTargetView::SurfaceTexture { .. } => {
        // note: the usage of surface texture could only contains TEXTURE_BINDING, so it's impossible
        // to do any read back from it. the upper layer should be draw content into temp texture for read back
        // and copy back to surface.
        return Err(ViewerRenderResultReadBackErr::UnableToReadSurfaceTexture);
      }
    };

    let mut encoder = self.device.create_encoder();

    let tex = GPU2DTextureView::try_from(tex).unwrap();

    let buffer = encoder.read_texture_2d::<f32>(
      &self.device,
      &tex,
      ReadRange {
        size: tex.size(),
        offset_x: 0,
        offset_y: 0,
      },
    );
    self.queue.submit_encoder(encoder);

    buffer.await.map_err(ViewerRenderResultReadBackErr::Gpu)
  }
}
