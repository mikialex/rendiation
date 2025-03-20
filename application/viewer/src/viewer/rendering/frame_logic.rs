use rendiation_algebra::*;
use rendiation_infinity_primitive::*;
use rendiation_texture_gpu_process::*;
use rendiation_webgpu::*;

use super::{
  axis::WorldCoordinateAxis, outline::ViewerOutlineSourceProvider, GridEffect, GridGround,
};
use crate::*;

pub struct ViewerFrameLogic {
  highlight: HighLighter,
  reproject: GPUReprojectInfo,
  taa: TAA,
  enable_ground: bool,
  enable_ssao: bool,
  enable_outline: bool,
  ssao: SSAO,
  _blur: CrossBlurData,
  ground: UniformBufferCachedDataView<ShaderPlane>,
  grid: UniformBufferCachedDataView<GridEffect>,
  post: UniformBufferCachedDataView<PostEffects>,
  pub axis: WorldCoordinateAxis,
}

impl ViewerFrameLogic {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      _blur: CrossBlurData::new(gpu),
      reproject: GPUReprojectInfo::new(gpu),
      taa: TAA::new(),
      enable_ground: true,
      enable_ssao: false,
      enable_outline: false,
      ssao: SSAO::new(gpu),
      ground: UniformBufferCachedDataView::create(&gpu.device, ShaderPlane::ground_like()),
      grid: UniformBufferCachedDataView::create_default(&gpu.device),
      post: UniformBufferCachedDataView::create_default(&gpu.device),
      axis: WorldCoordinateAxis::new(gpu),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    ui.checkbox(&mut self.enable_ground, "enable ground");
    ui.checkbox(&mut self.enable_ssao, "enable ssao");
    ui.checkbox(&mut self.enable_outline, "enable outline");
    post_egui(ui, &self.post);
  }

  #[must_use]
  #[instrument(name = "ViewerRasterizationFrameLogic rendering", skip_all)]
  pub fn render(
    &mut self,
    ctx: &mut FrameCtx,
    renderer: &dyn SceneRenderer<ContentKey = SceneContentKey>,
    lighting: &SceneLightSystem,
    tonemap: &ToneMap,
    content: &Viewer3dSceneCtx,
    final_target: &RenderTargetView,
    current_camera_view_projection_inv: Mat4<f32>,
    reversed_depth: bool,
    opaque_lighting: LightingTechniqueKind,
    deferred_mat_supports: &DeferLightingMaterialRegistry,
  ) -> RenderTargetView {
    let hdr_enabled = final_target.format() == TextureFormat::Rgba16Float;

    self
      .reproject
      .update(ctx, current_camera_view_projection_inv);

    self.post.upload_with_diff(&ctx.gpu.queue);

    let main_camera_gpu = renderer
      .get_camera_gpu()
      .make_component(content.main_camera)
      .unwrap();

    let taa_content = SceneCameraTAAContent {
      queue: &ctx.gpu.queue,
      camera: content.main_camera,
      renderer,
      f: |ctx: &mut FrameCtx| {
        let scene_result = attachment().use_hdr_if_enabled(hdr_enabled).request(ctx);
        let g_buffer = FrameGeometryBuffer::new(ctx);

        let (color_ops, depth_ops) = renderer.init_clear(content.scene);

        let mut background = renderer.render_background(
          content.scene,
          CameraRenderSource::Scene(content.main_camera),
          tonemap,
        );

        let _span = span!(Level::INFO, "main scene content encode pass");

        match opaque_lighting {
          LightingTechniqueKind::Forward => {
            let mut pass_base = pass("scene").with_color(&scene_result, color_ops);

            let g_buffer_base_writer = g_buffer.extend_pass_desc(&mut pass_base, depth_ops);
            let lighting = lighting.get_scene_forward_lighting_component(content.scene);

            let scene_pass_dispatcher = &RenderArray([
              &DefaultDisplayWriter as &dyn RenderComponent,
              &g_buffer_base_writer as &dyn RenderComponent,
              lighting.as_ref(),
            ]) as &dyn RenderComponent;

            let mut all_opaque_object = renderer.extract_and_make_pass_content(
              SceneContentKey::only_opaque_objects(),
              content.scene,
              CameraRenderSource::Scene(content.main_camera),
              ctx,
              scene_pass_dispatcher,
            );

            let mut all_transparent_object = renderer.extract_and_make_pass_content(
              SceneContentKey::only_alpha_blend_objects(),
              content.scene,
              CameraRenderSource::Scene(content.main_camera),
              ctx,
              scene_pass_dispatcher,
            );

            pass_base
              .render_ctx(ctx)
              // the following pass will check depth to decide if pixel is background,
              // so miss overwrite other channel is not a problem here
              .by(&mut background)
              .by(&mut all_opaque_object)
              .by(&mut all_transparent_object);
          }
          LightingTechniqueKind::DeferLighting => {
            let mut pass_base = pass("scene");

            let g_buffer_base_writer = g_buffer.extend_pass_desc(&mut pass_base, depth_ops);
            let mut m_buffer = FrameGeneralMaterialBuffer::new(ctx);

            let indices = m_buffer.extend_pass_desc(&mut pass_base);
            let material_writer = FrameGeneralMaterialBufferEncoder {
              indices,
              materials: deferred_mat_supports,
            };

            let scene_pass_dispatcher = &RenderArray([
              &g_buffer_base_writer as &dyn RenderComponent,
              &material_writer,
            ]) as &dyn RenderComponent;

            let mut main_scene_content = renderer.extract_and_make_pass_content(
              SceneContentKey::default(),
              content.scene,
              CameraRenderSource::Scene(content.main_camera),
              ctx,
              scene_pass_dispatcher,
            );

            pass_base.render_ctx(ctx).by(&mut main_scene_content);

            let geometry_from_g_buffer = Box::new(FrameGeometryBufferReconstructGeometryCtx {
              camera: &main_camera_gpu,
              g_buffer: &g_buffer,
            }) as Box<dyn GeometryCtxProvider>;
            let surface_from_m_buffer = Box::new(FrameGeneralMaterialBufferReconstructSurface {
              m_buffer: &m_buffer,
              registry: deferred_mat_supports,
            });
            let lighting = lighting.get_scene_lighting_component(
              content.scene,
              geometry_from_g_buffer,
              surface_from_m_buffer,
            );

            let lighting = RenderArray([
              &DefaultDisplayWriter as &dyn RenderComponent,
              lighting.as_ref(),
            ]);

            let _ = pass("deferred lighting compute")
              .with_color(&scene_result, color_ops)
              .render_ctx(ctx)
              .by(&mut background)
              .by(&mut lighting.draw_quad());
          }
        }

        if self.enable_ground {
          // this must a separate pass, because the id buffer should not be written.
          pass("grid_ground")
            .with_color(&scene_result, load())
            .with_depth(&g_buffer.depth, load())
            .render_ctx(ctx)
            .by(&mut GridGround {
              plane: &self.ground,
              shading: &self.grid,
              camera: main_camera_gpu.as_ref(),
              reversed_depth,
            });
        }

        if self.enable_ssao {
          let ao = self.ssao.draw(
            ctx,
            &g_buffer.depth,
            &self.reproject.reproject,
            reversed_depth,
          );

          pass("ao blend to scene")
            .with_color(&scene_result, load())
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
          NewTAAFrameSample {
            new_color: scene_result,
            new_depth: g_buffer.depth,
          },
          (g_buffer.entity_id, g_buffer.normal),
        )
      },
    };

    let (taa_result, scene_depth, (id_buffer, normal_buffer)) =
      self
        .taa
        .render_aa_content(taa_content, ctx, &self.reproject);
    let g_buffer = FrameGeometryBuffer {
      depth: scene_depth,
      normal: normal_buffer,
      entity_id: id_buffer,
    };

    let mut highlight_compose = (content.selected_target.is_some()).then(|| {
      let masked_content = renderer.render_models(
        Box::new(IteratorAsHostRenderBatch(content.selected_target)),
        CameraRenderSource::Scene(content.main_camera),
        &HighLightMaskDispatcher,
        ctx,
      );
      self.highlight.draw(ctx, masked_content)
    });

    let compose = pass("compose-all")
      .with_color(final_target, load())
      .render_ctx(ctx)
      .by(
        &mut PostProcess {
          input: taa_result.clone(),
          config: &self.post,
        }
        .draw_quad(),
      )
      .by(&mut highlight_compose);

    if self.enable_outline {
      // should we draw outline on taa buffer?
      compose.by(
        &mut OutlineComputer {
          source: &ViewerOutlineSourceProvider {
            g_buffer: &g_buffer,
            reproject: &self.reproject.reproject,
          },
        }
        .draw_quad_with_blend(BlendState::ALPHA_BLENDING.into()),
      );
    }

    g_buffer.entity_id
  }
}

struct SceneCameraTAAContent<'a, F> {
  renderer: &'a dyn SceneRenderer<ContentKey = SceneContentKey>,
  camera: EntityHandle<SceneCameraEntity>,
  queue: &'a GPUQueue,
  f: F,
}

impl<F, R> TAAContent<R> for SceneCameraTAAContent<'_, F>
where
  F: FnMut(&mut FrameCtx) -> (NewTAAFrameSample, R),
{
  fn set_jitter(&mut self, next_jitter: Vec2<f32>) {
    let cameras = self.renderer.get_camera_gpu();
    cameras.setup_camera_jitter(self.camera, next_jitter, self.queue);
  }

  fn render(&mut self, ctx: &mut FrameCtx) -> (NewTAAFrameSample, R) {
    (self.f)(ctx)
  }
}
