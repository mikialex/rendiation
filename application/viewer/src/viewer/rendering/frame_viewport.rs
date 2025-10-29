use rendiation_algebra::*;
use rendiation_infinity_primitive::*;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_shader_library::plane::ShaderPlaneUniform;
use rendiation_texture_gpu_process::*;
use rendiation_webgpu::*;

use super::{
  outline::ViewerOutlineSourceProvider, widget::WorldCoordinateAxis, GridEffect, GridGround,
};
use crate::*;

pub struct Viewer3dViewportRenderingCtx {
  highlight: HighLighter,
  reproject: GPUReprojectInfo,
  taa: TAA,
  pub enable_taa: bool,
  enable_fxaa: bool,
  enable_ground: bool,
  enable_ssao: bool,
  enable_outline: bool,
  outline_color: UniformBufferCachedDataView<Vec4<f32>>,
  outline_background_color: Vec3<f32>,
  show_outline_only: bool,
  ssao: SSAO,
  _blur: CrossBlurData,
  ground: UniformBufferCachedDataView<ShaderPlaneUniform>,
  grid: UniformBufferCachedDataView<GridEffect>,
  post: UniformBufferCachedDataView<PostEffects>,
  pub axis: WorldCoordinateAxis,
  rtx_rendering_enabled: bool,
  rtx_effect_mode: RayTracingEffectMode,
  pub transparent_config: ViewerTransparentContentRenderStyle,
  on_encoding_finished: EventSource<ViewportRenderedResult>,
  expect_read_back_for_next_render_result: bool,
  pub picker: GPUxEntityIdMapPicker,
  request_reset_rtx_sample: bool,
  pub oit: ViewerTransparentRenderer,
  pub rtx_ao: Option<SceneRayTracingAORenderer>,
  pub rtx_pt: Option<DeviceReferencePathTracingRenderer>,

  pub(super) enable_on_demand_rendering: bool,
  pub(super) on_demand_rendering_cached_frame: Option<RenderTargetView>,
  pub(super) not_any_changed_frame_count: u32,

  rendered_camera: Option<EntityHandle<SceneCameraEntity>>,
}

impl Viewer3dViewportRenderingCtx {
  pub fn new(gpu: &GPU, init_config: &ViewerInitConfig) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      _blur: CrossBlurData::new(gpu),
      reproject: GPUReprojectInfo::new(gpu),
      taa: TAA::new(),
      enable_taa: true,
      enable_fxaa: false,
      enable_ground: true,
      enable_ssao: false,
      enable_outline: false,
      ssao: SSAO::new(gpu),
      outline_color: UniformBufferCachedDataView::create(&gpu.device, vec4(0., 0., 0., 1.)),
      outline_background_color: vec3(1., 1., 1.),
      show_outline_only: false,
      ground: UniformBufferCachedDataView::create(&gpu.device, ground_like_shader_plane()),
      grid: UniformBufferCachedDataView::create_default(&gpu.device),
      post: UniformBufferCachedDataView::create_default(&gpu.device),
      axis: WorldCoordinateAxis::new(gpu),
      on_encoding_finished: Default::default(),
      expect_read_back_for_next_render_result: false,
      picker: Default::default(),
      transparent_config: init_config.transparent_config,
      rtx_effect_mode: RayTracingEffectMode::ReferenceTracing,
      rtx_rendering_enabled: false,
      request_reset_rtx_sample: true,
      enable_on_demand_rendering: init_config.enable_on_demand_rendering,
      on_demand_rendering_cached_frame: None,
      not_any_changed_frame_count: 0,
      oit: ViewerTransparentRenderer::NaiveAlphaBlend,
      rtx_ao: None,
      rtx_pt: None,
      rendered_camera: None,
    }
  }

  pub fn egui(&mut self, ui: &mut UiWithChangeInfo, rtx_renderer_enabled: bool) {
    ui.checkbox(
      &mut self.enable_on_demand_rendering,
      "enable_on_demand_rendering",
    );

    ui.checkbox(&mut self.enable_taa, "enable taa");
    ui.checkbox(&mut self.enable_fxaa, "enable fxaa");
    if self.enable_fxaa && self.enable_taa {
      ui.label("enable fxaa with other aa method is allowed, but may have undesirable result");
    }
    ui.checkbox(&mut self.enable_ground, "enable ground");
    ui.checkbox(&mut self.enable_ssao, "enable ssao");

    ui.collapsing("outline", |ui| {
      ui.checkbox(&mut self.enable_outline, "enable outline");
      ui.checkbox(&mut self.show_outline_only, "show_outline_only");
      self.outline_color.mutate(|color| {
        modify_color4_change(ui, color);
      });
      modify_color_change(ui, &mut self.outline_background_color);
    });

    egui::ComboBox::from_label("how to render transparent objects?")
      .selected_text(format!("{:?}", &self.transparent_config,))
      .show_ui_changed(ui, |ui| {
        ui.selectable_value(
          &mut self.transparent_config,
          ViewerTransparentContentRenderStyle::NaiveAlphaBlend,
          "naive alpha blend",
        );

        ui.selectable_value(
          &mut self.transparent_config,
          ViewerTransparentContentRenderStyle::WeightedOIT,
          "oit weighted style",
        );

        ui.selectable_value(
          &mut self.transparent_config,
          ViewerTransparentContentRenderStyle::Loop32OIT,
          "oit loop32 style",
        )
      });

    self.oit = match self.transparent_config {
      ViewerTransparentContentRenderStyle::NaiveAlphaBlend => {
        ViewerTransparentRenderer::NaiveAlphaBlend
      }
      ViewerTransparentContentRenderStyle::Loop32OIT => ViewerTransparentRenderer::Loop32OIT(
        Arc::new(RwLock::new(rendiation_oit::OitLoop32Renderer::new(4))),
      ),
      ViewerTransparentContentRenderStyle::WeightedOIT => ViewerTransparentRenderer::WeightedOIT,
    };

    if rtx_renderer_enabled {
      if ui
        .checkbox(&mut self.rtx_rendering_enabled, "enable ray tracing")
        .changed()
      {
        self.request_reset_rtx_sample = true;
      }

      if !self.rtx_rendering_enabled {
        self.rtx_ao = None;
        self.rtx_pt = None;
      }

      egui::ComboBox::from_label("ray tracing mode")
        .selected_text(format!("{:?}", &self.rtx_effect_mode))
        .show_ui_changed(ui, |ui| {
          ui.selectable_value(
            &mut self.rtx_effect_mode,
            RayTracingEffectMode::ReferenceTracing,
            "Path tracing",
          );
          ui.selectable_value(
            &mut self.rtx_effect_mode,
            RayTracingEffectMode::AO,
            "AO only",
          );
        });

      if ui.button("reset  sample").clicked() {
        self.request_reset_rtx_sample = true;
      }
    }

    post_egui(ui, &self.post);
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

  pub fn check_should_render_and_copy_cached(
    &mut self,
    target: &RenderTargetView,
    viewport: &ViewerViewPort,
    any_changed: bool,
    ctx: &mut FrameCtx,
  ) -> bool {
    if let Some(camera) = self.rendered_camera {
      if camera != viewport.camera {
        self.on_demand_rendering_cached_frame = None;
      }
    }

    // currently the rtx mode is offline style, so we need continually rendering
    if self.rtx_rendering_enabled {
      self.on_demand_rendering_cached_frame = None;
    }

    if !self.enable_on_demand_rendering {
      self.on_demand_rendering_cached_frame = None;
    }

    if any_changed {
      self.on_demand_rendering_cached_frame = None;
      self.not_any_changed_frame_count = 0;
    } else {
      self.not_any_changed_frame_count += 1;
    }

    if self.enable_taa && self.not_any_changed_frame_count <= 32 {
      self.on_demand_rendering_cached_frame = None;
    }

    if let Some(cached_frame) = &self.on_demand_rendering_cached_frame {
      if cached_frame.size() != target.size() {
        self.on_demand_rendering_cached_frame = None;
      }
    }

    if let Some(cached_frame) = &self.on_demand_rendering_cached_frame {
      pass("on demand rendering copy cached frame")
        .with_color(target, store_full_frame())
        .render_ctx(ctx)
        .by(&mut rendiation_texture_gpu_process::copy_frame(
          cached_frame.clone(),
          None,
        ));

      false
    } else {
      true
    }
  }

  pub fn render(
    &mut self,
    ctx: &mut FrameCtx,
    renderer: &ViewerRendererInstance,
    content: &Viewer3dContent,
    viewport_idx: usize,
    final_target: &RenderTargetView,
    waker: &Waker,
  ) {
    let viewport = &content.viewports[viewport_idx];
    let camera = viewport.camera;

    let should_do_extra_copy = self.should_do_extra_copy(final_target, viewport);
    let render_target = if should_do_extra_copy {
      // we do extra copy in this case, so we have to make sure the copy source has correct usage
      let mut key = final_target.create_attachment_key();
      key.usage |= TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;

      let viewport_size =
        Size::from_u32_pair_min_one(viewport.viewport.zw().map(|v| v as u32).into());
      key.size = viewport_size;
      key.request(ctx)
    } else {
      final_target.clone()
    };

    if self.rtx_rendering_enabled {
      self.render_ray_tracing(
        ctx,
        renderer,
        content,
        &render_target,
        camera,
        renderer.lighting.tonemap,
      );
    } else {
      self.render_raster(ctx, renderer, content, &render_target, camera, waker);
    }

    {
      let main_camera_gpu = renderer.camera.make_component(camera).unwrap();

      let widgets_result = draw_widgets(
        ctx,
        renderer.raster_scene_renderer.as_ref(),
        &renderer.extractor,
        content.widget_scene,
        renderer.reversed_depth,
        &main_camera_gpu,
        &self.axis,
      );
      let mut copy_scene_msaa_widgets = copy_frame(
        widgets_result,
        BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
      );
      pass("copy_scene_msaa_widgets")
        .with_color(&render_target, load_and_store())
        .render_ctx(ctx)
        .by(&mut copy_scene_msaa_widgets);
    }

    // do extra copy to surface texture
    if should_do_extra_copy {
      pass("copy frame renderer local to surface")
        .with_color(final_target, store_full_frame())
        .render_ctx(ctx)
        .by(
          &mut CopyFrame {
            source: render_target.clone(),
            viewport: viewport.viewport.into(),
          }
          .draw_quad(),
        );
    }
    self.expect_read_back_for_next_render_result = false;

    if self.should_do_frame_caching() {
      let mut key = final_target.create_attachment_key();
      key.usage |= TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
      let frame_cache = key.request(ctx);
      pass("copy rendered result to cached frame")
        .with_color(&frame_cache, store_full_frame())
        .render_ctx(ctx)
        .by(&mut copy_frame(render_target.clone(), None));
      self.on_demand_rendering_cached_frame = Some(frame_cache);
    }

    self.on_encoding_finished.emit(&ViewportRenderedResult {
      target: render_target,
      device: ctx.gpu.device.clone(),
      queue: ctx.gpu.queue.clone(),
    });

    self.rendered_camera = camera.into();
  }

  fn render_ray_tracing(
    &mut self,
    ctx: &mut FrameCtx,
    renderer: &ViewerRendererInstance,
    content: &Viewer3dContent,
    final_target: &RenderTargetView,
    camera: EntityHandle<SceneCameraEntity>,
    tonemap: &ToneMap,
  ) {
    if let Some((rtx_renderer, core)) = &renderer.rtx_system {
      match self.rtx_effect_mode {
        RayTracingEffectMode::AO => {
          let ao = self
            .rtx_ao
            .get_or_insert_with(|| SceneRayTracingAORenderer::new(core, ctx.gpu));
          if self.request_reset_rtx_sample || rtx_renderer.base.1 || rtx_renderer.ao.1 {
            ao.reset_sample();
          }

          let ao_result = ao.render(
            ctx,
            core.rtx_system.as_ref(),
            &rtx_renderer.base.0,
            content.scene,
            camera,
            &rtx_renderer.ao.0,
          );

          pass("copy rtx ao into final target")
            .with_color(final_target, store_full_frame())
            .render_ctx(ctx)
            .by(&mut copy_frame(RenderTargetView::from(ao_result), None));
        }
        RayTracingEffectMode::ReferenceTracing => {
          let pt = self
            .rtx_pt
            .get_or_insert_with(|| DeviceReferencePathTracingRenderer::new(core, ctx.gpu));
          if self.request_reset_rtx_sample || rtx_renderer.base.1 || rtx_renderer.pt.1 {
            pt.reset_sample();
          }

          let result = pt.render(
            ctx,
            core.rtx_system.as_ref(),
            &rtx_renderer.base.0,
            content.scene,
            camera,
            tonemap,
            &renderer.background,
            &rtx_renderer.pt.0,
          );
          pass("copy pt result into final target")
            .with_color(final_target, store_full_frame())
            .render_ctx(ctx)
            .by(&mut copy_frame(RenderTargetView::from(result), None));
        }
      }
    }

    self.picker.notify_frame_id_buffer_not_available();
    self.request_reset_rtx_sample = false;
  }

  fn render_raster(
    &mut self,
    ctx: &mut FrameCtx,
    renderer: &ViewerRendererInstance,
    content: &Viewer3dContent,
    render_target: &RenderTargetView,
    camera: EntityHandle<SceneCameraEntity>,
    waker: &Waker,
  ) {
    let camera_transform = renderer.camera_transforms.access(&camera).unwrap();
    let current_view_projection_inv = camera_transform.view_projection_inv;
    self.reproject.update(ctx, current_view_projection_inv);
    if let Some(mesh_lod_graph_renderer) = &renderer.mesh_lod_graph_renderer {
      if camera_transform
        .projection
        .check_is_perspective_matrix_assume_common_projection()
      {
        mesh_lod_graph_renderer.setup_lod_decider(
          ctx.gpu,
          camera_transform.projection,
          camera_transform.world,
          render_target.size().into_f32().into(),
        );
      }
    }

    let hdr_enabled = render_target.format() == TextureFormat::Rgba16Float;

    self.post.upload_with_diff(&ctx.gpu.queue);
    self.outline_color.upload_with_diff(&ctx.gpu.queue);
    let is_outline_only_mode = self.is_outline_only_mode();

    let camera_gpu = renderer.camera.make_component(camera).unwrap();

    let renderer_c = ViewerSceneRenderer {
      scene: renderer.raster_scene_renderer.as_ref(),
      batch_extractor: &renderer.extractor,
      cameras: &renderer.camera,
      background: &renderer.background,
      oit: self.oit.clone(),
      reversed_depth: renderer.reversed_depth,
      camera_transforms: &renderer.camera_transforms,
      sm_world_bounding: &renderer.sm_world_bounding,
    };

    let mut taa_content = SceneCameraTAAContent {
      queue: &ctx.gpu.queue,
      camera,
      renderer: &renderer_c,
      f: |ctx: &mut FrameCtx| {
        let scene_result = attachment().use_hdr_if_enabled(hdr_enabled).request(ctx);
        let g_buffer = FrameGeometryBuffer::new(ctx);

        let _span = span!(Level::INFO, "main scene content encode pass");

        render_lighting_scene_content(
          ctx,
          &renderer.lighting,
          &renderer.culling,
          &renderer_c,
          content.scene,
          camera,
          &scene_result,
          &g_buffer,
          !is_outline_only_mode,
        );

        if self.enable_ground && !is_outline_only_mode {
          // this must a separate pass, because the id buffer should not be written.
          pass("grid_ground")
            .with_color(&scene_result, load_and_store())
            .with_depth(&g_buffer.depth, load_and_store())
            .render_ctx(ctx)
            .by(&mut GridGround {
              plane: &self.ground,
              shading: &self.grid,
              camera: &camera_gpu,
              reversed_depth: renderer.reversed_depth,
            });
        }

        if self.enable_ssao {
          let ao = self.ssao.draw(
            ctx,
            &g_buffer.depth,
            &self.reproject.reproject,
            renderer.reversed_depth,
          );

          pass("ao blend to scene")
            .with_color(&scene_result, load_and_store())
            .render_ctx(ctx)
            .by(&mut copy_frame(
              ao,
              BlendState {
                color: BlendComponent {
                  src_factor: BlendFactor::Dst,
                  dst_factor: BlendFactor::Zero,
                  operation: BlendOperation::Add,
                },
                alpha: BlendComponent::REPLACE,
              }
              .into(),
            ));
        }

        (
          TAAFrame {
            color: scene_result,
            depth: g_buffer.depth,
          },
          (g_buffer.entity_id, g_buffer.normal),
        )
      },
    };

    let (
      TAAFrame {
        color: maybe_aa_result,
        depth: scene_depth,
      },
      (id_buffer, normal_buffer),
    ) = if self.enable_taa {
      self
        .taa
        .render_aa_content(taa_content, ctx, &self.reproject)
    } else {
      taa_content.render(ctx)
    };

    let maybe_aa_result = if self.enable_fxaa {
      let fxaa_target = maybe_aa_result.create_attachment_key().request(ctx);

      pass("fxaa")
        .with_color(&fxaa_target, store_full_frame())
        .render_ctx(ctx)
        .by(
          &mut FXAA {
            source: &maybe_aa_result,
          }
          .draw_quad(),
        );

      fxaa_target
    } else {
      maybe_aa_result
    };

    let g_buffer = FrameGeometryBuffer {
      depth: scene_depth,
      normal: normal_buffer,
      entity_id: id_buffer,
    };

    let mut post_process = (!is_outline_only_mode).then(|| {
      PostProcess {
        input: maybe_aa_result.clone(),
        config: &self.post,
        target_is_srgb: render_target.format().is_srgb(),
      }
      .draw_quad()
    });

    let mut highlight_compose = (content.selected_target.is_some()).then(|| {
      let batch = Box::new(IteratorAsHostRenderBatch(content.selected_target));
      let batch = SceneModelRenderBatch::Host(batch);
      let masked_content = renderer
        .raster_scene_renderer
        .make_scene_batch_pass_content(batch, &camera_gpu, &HighLightMaskDispatcher, ctx);
      self.highlight.draw(ctx, masked_content)
    });

    let pass_init = if is_outline_only_mode {
      let c = self.outline_background_color;
      clear_and_store(color(c.x as f64, c.y as f64, c.z as f64, 1.0))
    } else {
      store_full_frame()
    };

    let mut compose = pass("compose-all")
      .with_color(render_target, pass_init)
      .render_ctx(ctx)
      .by_if(&mut post_process)
      .by_if(&mut highlight_compose);

    // the outline will not draw on taa frame, because the effect is screen space
    if self.enable_outline {
      compose = compose.by(
        &mut OutlineComputer {
          source: &ViewerOutlineSourceProvider {
            g_buffer: &g_buffer,
            reproject: &self.reproject.reproject,
            outline_color: &self.outline_color,
          },
        }
        .draw_quad_with_alpha_blending(),
      );
    }

    drop(compose);

    let entity_id = g_buffer
      .entity_id
      .expect_standalone_common_texture_view()
      .clone();

    self.picker.read_new_frame_id_buffer(
      &GPUTypedTextureView::<TextureDimension2, u32>::try_from(entity_id).unwrap(),
      ctx.gpu,
      &mut ctx.encoder,
      waker,
    );
  }

  fn should_do_frame_caching(&self) -> bool {
    self.enable_on_demand_rendering && !self.rtx_rendering_enabled
  }

  fn should_do_extra_copy(
    &self,
    final_target: &RenderTargetView,
    viewport: &ViewerViewPort,
  ) -> bool {
    let viewport_size =
      Size::from_u32_pair_min_one(viewport.viewport.zw().map(|v| v as u32).into());
    let is_full_covered = viewport_size == final_target.size(); // todo，should check if in bound, not size

    !is_full_covered
      || (self.expect_read_back_for_next_render_result || self.should_do_frame_caching())
        && matches!(final_target, RenderTargetView::SurfaceTexture { .. })
  }

  fn is_outline_only_mode(&self) -> bool {
    self.enable_outline && self.show_outline_only
  }
}

pub struct ViewerSceneRenderer<'a> {
  pub scene: &'a dyn SceneRenderer,
  pub batch_extractor: &'a DefaultSceneBatchExtractor,
  pub cameras: &'a CameraRenderer,
  pub background: &'a SceneBackgroundRenderer,
  pub oit: ViewerTransparentRenderer,
  pub reversed_depth: bool,
  pub camera_transforms: &'a BoxedDynQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub sm_world_bounding: &'a BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
}

struct SceneCameraTAAContent<'a, F> {
  renderer: &'a ViewerSceneRenderer<'a>,
  camera: EntityHandle<SceneCameraEntity>,
  queue: &'a GPUQueue,
  f: F,
}

impl<F, R> TAAContent<R> for SceneCameraTAAContent<'_, F>
where
  F: FnMut(&mut FrameCtx) -> (TAAFrame, R),
{
  fn set_jitter(&mut self, next_jitter: Vec2<f32>) {
    self
      .renderer
      .cameras
      .setup_camera_jitter(self.camera, next_jitter, self.queue);
  }

  fn render(&mut self, ctx: &mut FrameCtx) -> (TAAFrame, R) {
    (self.f)(ctx)
  }
}

#[derive(Clone)]
struct ViewportRenderedResult {
  target: RenderTargetView,
  device: GPUDevice,
  queue: GPUQueue,
}

#[derive(Debug)]
pub enum ViewerRenderResultReadBackErr {
  Gpu(rendiation_webgpu::BufferAsyncError),
  UnableToReadSurfaceTexture,
}

impl ViewportRenderedResult {
  async fn read(self) -> Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr> {
    let tex = match self.target {
      RenderTargetView::Texture(tex) => tex.clone(),
      RenderTargetView::ReusedTexture(tex) => tex.item().clone(),
      RenderTargetView::SurfaceTexture { .. } => {
        // note: the usage of surface texture could only guaranteed contains RENDER_ATTACHMENT,
        // so it's maybe impossible to do any read back from it. the upper layer should be draw
        // content into temp texture for read back and copy back to surface.
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
