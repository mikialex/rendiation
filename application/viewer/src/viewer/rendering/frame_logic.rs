use rendiation_algebra::*;
use rendiation_infinity_primitive::*;
use rendiation_shader_library::plane::ShaderPlaneUniform;
use rendiation_texture_gpu_process::*;
use rendiation_webgpu::*;

use super::{
  outline::ViewerOutlineSourceProvider, widget::WorldCoordinateAxis, GridEffect, GridGround,
};
use crate::*;

pub struct ViewerFrameLogic {
  highlight: HighLighter,
  reproject: GPUReprojectInfo,
  taa: TAA,
  enable_taa: bool,
  enable_fxaa: bool,
  enable_ground: bool,
  enable_ssao: bool,
  enable_outline: bool,
  ssao: SSAO,
  _blur: CrossBlurData,
  ground: UniformBufferCachedDataView<ShaderPlaneUniform>,
  grid: UniformBufferCachedDataView<GridEffect>,
  post: UniformBufferCachedDataView<PostEffects>,
  pub axis: WorldCoordinateAxis,
}

pub struct ViewerSceneRenderer<'a> {
  pub scene: &'a dyn SceneRenderer,
  pub batch_extractor: &'a DefaultSceneBatchExtractor,
  pub cameras: &'a CameraRenderer,
  pub background: &'a SceneBackgroundRenderer,
  pub oit: ViewerTransparentRenderer,
  pub reversed_depth: bool,
}

impl ViewerFrameLogic {
  pub fn new(gpu: &GPU) -> Self {
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
      ground: UniformBufferCachedDataView::create(&gpu.device, ground_like_shader_plane()),
      grid: UniformBufferCachedDataView::create_default(&gpu.device),
      post: UniformBufferCachedDataView::create_default(&gpu.device),
      axis: WorldCoordinateAxis::new(gpu),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    ui.checkbox(&mut self.enable_taa, "enable taa");
    ui.checkbox(&mut self.enable_fxaa, "enable fxaa");
    if self.enable_fxaa && self.enable_taa {
      ui.label("enable fxaa with other aa method is allowed, but may have undesirable result");
    }
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
    renderer: &ViewerSceneRenderer,
    culling: &ViewerCulling,
    scene_derive: &Viewer3dSceneDerive,
    lighting: &LightingRenderingCx,
    content: &Viewer3dSceneCtx,
    final_target: &RenderTargetView,
    current_camera_view_projection_inv: Mat4<f64>,
    reversed_depth: bool,
  ) -> RenderTargetView {
    let hdr_enabled = final_target.format() == TextureFormat::Rgba16Float;

    self
      .reproject
      .update(ctx, current_camera_view_projection_inv);

    self.post.upload_with_diff(&ctx.gpu.queue);

    let main_camera_gpu = renderer
      .cameras
      .make_component(content.main_camera)
      .unwrap();

    let mut taa_content = SceneCameraTAAContent {
      queue: &ctx.gpu.queue,
      camera: content.main_camera,
      renderer,
      f: |ctx: &mut FrameCtx| {
        let scene_result = attachment().use_hdr_if_enabled(hdr_enabled).request(ctx);
        let g_buffer = FrameGeometryBuffer::new(ctx);

        let _span = span!(Level::INFO, "main scene content encode pass");

        render_lighting_scene_content(
          ctx,
          lighting,
          culling,
          renderer,
          content,
          scene_derive,
          &scene_result,
          &g_buffer,
        );

        if self.enable_ground {
          // this must a separate pass, because the id buffer should not be written.
          pass("grid_ground")
            .with_color(&scene_result, load_and_store())
            .with_depth(&g_buffer.depth, load_and_store())
            .render_ctx(ctx)
            .by(&mut GridGround {
              plane: &self.ground,
              shading: &self.grid,
              camera: &main_camera_gpu,
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

    let mut highlight_compose = (content.selected_target.is_some()).then(|| {
      let batch = Box::new(IteratorAsHostRenderBatch(content.selected_target));
      let batch = SceneModelRenderBatch::Host(batch);
      let masked_content = renderer.scene.make_scene_batch_pass_content(
        batch,
        &main_camera_gpu,
        &HighLightMaskDispatcher,
        ctx,
      );
      self.highlight.draw(ctx, masked_content)
    });

    let compose = pass("compose-all")
      .with_color(final_target, store_full_frame())
      .render_ctx(ctx)
      .by(
        &mut PostProcess {
          input: maybe_aa_result.clone(),
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
        .draw_quad(),
      );
    }

    g_buffer.entity_id
  }
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
