use crate::*;

mod axis;
mod frame_logic;
mod grid_ground;
mod lighting;
mod outline;

mod g_buffer;
pub use g_buffer::*;
mod defer_lighting;
pub use defer_lighting::*;

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

#[derive(Debug, PartialEq, Clone, Copy)]
enum RasterizationRenderBackendType {
  Gles,
  Indirect,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LightingTechniqueKind {
  Forward,
  DeferLighting,
  // Visibility,
}

pub type BoxedSceneRenderImplProvider =
  Box<dyn RenderImplProvider<Box<dyn SceneRenderer<ContentKey = SceneContentKey>>>>;

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

fn init_renderer(
  updater: &mut ReactiveQueryJoinUpdater,
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

  renderer_impl.register_resource(updater, gpu);
  renderer_impl
}

pub struct Viewer3dRenderingCtx {
  ndc: ViewerNDC,
  frame_logic: ViewerFrameLogic,
  rendering_resource: ReactiveQueryJoinUpdater,
  renderer_impl: BoxedSceneRenderImplProvider,
  indirect_occlusion_culling_impl: Option<GPUTwoPassOcclusionCulling>,
  current_renderer_impl_ty: RasterizationRenderBackendType,
  rtx_ao_renderer_impl: Option<RayTracingAORenderSystem>,
  enable_rtx_ao_rendering: bool,
  opaque_scene_content_lighting_technique: LightingTechniqueKind,
  lighting: LightSystem,
  material_defer_lighting_supports: DeferLightingMaterialRegistry,
  pool: AttachmentPool,
  gpu: GPU,
  on_encoding_finished: EventSource<ViewRenderedState>,
  expect_read_back_for_next_render_result: bool,
  current_camera_view_projection_inv: Mat4<f32>,
  camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
}

impl Viewer3dRenderingCtx {
  pub fn gpu(&self) -> &GPU {
    &self.gpu
  }

  pub fn new(
    gpu: GPU,
    ndc: ViewerNDC,
    camera_source: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  ) -> Self {
    let mut rendering_resource = ReactiveQueryJoinUpdater::default();

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
      ndc,
      indirect_occlusion_culling_impl: None,
      rendering_resource,
      renderer_impl,
      current_renderer_impl_ty: RasterizationRenderBackendType::Gles,
      rtx_ao_renderer_impl: None, // late init
      enable_rtx_ao_rendering: false,
      opaque_scene_content_lighting_technique: LightingTechniqueKind::Forward,
      frame_logic: ViewerFrameLogic::new(&gpu),
      lighting,
      material_defer_lighting_supports: Default::default(),
      gpu,
      pool: Default::default(),
      on_encoding_finished: Default::default(),
      expect_read_back_for_next_render_result: false,
      current_camera_view_projection_inv: Default::default(),
      camera_source: camera_source_init,
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

  pub fn set_enable_rtx_ao_rendering_support(&mut self, enable: bool) {
    if enable {
      if self.rtx_ao_renderer_impl.is_none() {
        let rtx_backend_system = GPUWaveFrontComputeRaytracingSystem::new(&self.gpu);
        let rtx_system = RtxSystemCore::new(Box::new(rtx_backend_system));
        let mut rtx_ao_renderer_impl = RayTracingAORenderSystem::new(
          &rtx_system,
          &self.gpu,
          self.camera_source.clone_as_static(),
        );

        rtx_ao_renderer_impl.register_resource(&mut self.rendering_resource, &self.gpu);

        self.rtx_ao_renderer_impl = Some(rtx_ao_renderer_impl);
      }
    } else {
      if let Some(rtx) = &mut self.rtx_ao_renderer_impl {
        rtx.deregister_resource(&mut self.rendering_resource);
      }
      self.rtx_ao_renderer_impl = None;
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    let is_target_support_indirect_draw = self
      .gpu
      .info
      .supported_features
      .contains(Features::MULTI_DRAW_INDIRECT_COUNT);

    let old = self.current_renderer_impl_ty;
    egui::ComboBox::from_label("RasterizationRender Backend")
      .selected_text(format!("{:?}", &self.current_renderer_impl_ty))
      .show_ui(ui, |ui| {
        ui.selectable_value(
          &mut self.current_renderer_impl_ty,
          RasterizationRenderBackendType::Gles,
          "Gles",
        );

        ui.add_enabled_ui(is_target_support_indirect_draw, |ui| {
          ui.selectable_value(
            &mut self.current_renderer_impl_ty,
            RasterizationRenderBackendType::Indirect,
            "Indirect",
          )
          .on_disabled_hover_text("current platform/gpu does not support indirect rendering");
        });
      });

    ui.separator();

    egui::ComboBox::from_label("Lighting technique for opaque objects")
      .selected_text(format!(
        "{:?}",
        &self.opaque_scene_content_lighting_technique
      ))
      .show_ui(ui, |ui| {
        ui.selectable_value(
          &mut self.opaque_scene_content_lighting_technique,
          LightingTechniqueKind::Forward,
          "Forward",
        );

        ui.selectable_value(
          &mut self.opaque_scene_content_lighting_technique,
          LightingTechniqueKind::DeferLighting,
          "DeferLighting",
        )
      });

    ui.separator();

    if old != self.current_renderer_impl_ty {
      self
        .renderer_impl
        .deregister_resource(&mut self.rendering_resource);
      self.renderer_impl = init_renderer(
        &mut self.rendering_resource,
        self.current_renderer_impl_ty,
        &self.gpu,
        self.camera_source.clone_as_static(),
        self.ndc.enable_reverse_z,
      );
    }

    ui.add_enabled_ui(is_target_support_indirect_draw, |ui| {
      let mut indirect_occlusion_culling_impl_exist =
        self.indirect_occlusion_culling_impl.is_some();
      ui.checkbox(
        &mut indirect_occlusion_culling_impl_exist,
        "occlusion_culling_system_is_ready",
      )
      .on_disabled_hover_text("current platform/gpu does not support gpu driven occlusion culling");
      self.set_enable_indirect_occlusion_culling_support(indirect_occlusion_culling_impl_exist);
    });

    let is_vulkan = self.gpu.info.adaptor_info.backend == Backend::Vulkan;
    ui.add_enabled_ui(is_vulkan, |ui| {
      let mut rtx_ao_renderer_impl_exist = self.rtx_ao_renderer_impl.is_some();
      ui.checkbox(&mut rtx_ao_renderer_impl_exist, "rtx_ao_renderer_is_ready")
        .on_disabled_hover_text(
          "currently the ray tracing related feature only enabled on vulkan backend",
        );
      self.set_enable_rtx_ao_rendering_support(rtx_ao_renderer_impl_exist);

      if let Some(ao) = &self.rtx_ao_renderer_impl {
        ui.checkbox(&mut self.enable_rtx_ao_rendering, "enable_rtx_ao_rendering");
        // todo, currently the on demand rendering is broken, use this button to workaround.
        if ui.button("reset ao sample").clicked() {
          ao.reset_ao_sample(&self.gpu);
        }
      }
    });

    ui.separator();

    self.lighting.egui(ui);
    self.frame_logic.egui(ui);
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

  pub fn resize_view(&mut self) {
    self.pool.clear();
  }

  pub fn update_next_render_camera_info(&mut self, camera_view_projection_inv: Mat4<f32>) {
    self.current_camera_view_projection_inv = camera_view_projection_inv;
  }

  #[instrument(name = "frame rendering", skip_all)]
  pub fn render(&mut self, target: RenderTargetView, content: &Viewer3dSceneCtx, cx: &mut Context) {
    let span = span!(Level::INFO, "update all rendering resource");
    let mut resource = self.rendering_resource.poll_update_all(cx);
    drop(span);

    let renderer = self.renderer_impl.create_impl(&mut resource);

    let render_target = if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      RenderTargetView::Texture(create_empty_2d_texture_view(
        &self.gpu,
        target.size(),
        basic_texture_usages(),
        target.format(),
      ))
    } else {
      target.clone()
    };

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool);

    if self.enable_rtx_ao_rendering && self.rtx_ao_renderer_impl.is_some() {
      let mut rtx_ao_renderer = self
        .rtx_ao_renderer_impl
        .as_ref()
        .unwrap()
        .create_impl(&mut resource);

      let ao_result = rtx_ao_renderer.render(&mut ctx, content.scene, content.main_camera);

      pass("copy rtx ao into final target")
        .with_color(target.clone(), load())
        .render_ctx(&mut ctx)
        .by(&mut copy_frame(
          AttachmentView::from_any_view(ao_result),
          None,
        ));
    } else {
      let lighting = self.lighting.prepare_and_create_impl(
        &mut resource,
        &mut ctx,
        cx,
        renderer.as_ref(),
        content.scene,
      );

      self.frame_logic.render(
        &mut ctx,
        renderer.as_ref(),
        &lighting,
        content,
        &render_target,
        self.current_camera_view_projection_inv,
        self.ndc.enable_reverse_z,
        self.opaque_scene_content_lighting_technique,
        &self.material_defer_lighting_supports,
      );
    }

    // do extra copy to surface texture
    if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      pass("extra final copy to surface")
        .with_color(target, load())
        .render_ctx(&mut ctx)
        .by(&mut rendiation_texture_gpu_process::copy_frame(
          AttachmentView::from_any_view(render_target.clone()),
          None,
        ));
    }
    self.expect_read_back_for_next_render_result = false;
    ctx.final_submit();

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
    match self.target {
      RenderTargetView::Texture(tex) => {
        // I have to write this, because I don't know why compiler can't known the encoder is
        // dropped and will not across the await point
        let buffer = {
          let mut encoder = self.device.create_encoder();

          let buffer = encoder.read_texture_2d(
            &self.device,
            &tex.resource.clone().try_into().unwrap(),
            ReadRange {
              size: Size::from_u32_pair_min_one((
                tex.resource.desc.size.width,
                tex.resource.desc.size.height,
              )),
              offset_x: 0,
              offset_y: 0,
            },
          );
          self.queue.submit_encoder(encoder);
          buffer
        };

        buffer.await.map_err(ViewerRenderResultReadBackErr::Gpu)
      }
      RenderTargetView::SurfaceTexture { .. } => {
        // note: the usage of surface texture could only contains TEXTURE_BINDING, so it's impossible
        // to do any read back from it. the upper layer should be draw content into temp texture for read back
        // and copy back to surface.
        Err(ViewerRenderResultReadBackErr::UnableToReadSurfaceTexture)
      }
    }
  }
}
