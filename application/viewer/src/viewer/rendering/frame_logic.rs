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
  enable_ssao: bool,
  enable_outline: bool,
  ssao: SSAO,
  _blur: CrossBlurData,
  ground: UniformBufferCachedDataView<ShaderPlane>,
  grid: UniformBufferCachedDataView<GridEffect>,
  post: UniformBufferCachedDataView<PostEffects>,
  axis: WorldCoordinateAxis,
}

impl ViewerFrameLogic {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      _blur: CrossBlurData::new(gpu),
      reproject: GPUReprojectInfo::new(gpu),
      taa: TAA::new(),
      enable_ssao: true,
      enable_outline: false,
      ssao: SSAO::new(gpu),
      ground: UniformBufferCachedDataView::create(&gpu.device, ShaderPlane::ground_like()),
      grid: UniformBufferCachedDataView::create_default(&gpu.device),
      post: UniformBufferCachedDataView::create_default(&gpu.device),
      axis: WorldCoordinateAxis::new(gpu),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    ui.checkbox(&mut self.enable_ssao, "enable ssao");
    ui.checkbox(&mut self.enable_outline, "enable outline");
    post_egui(ui, &self.post);
  }

  #[instrument(name = "ViewerRasterizationFrameLogic rendering", skip_all)]
  pub fn render(
    &mut self,
    ctx: &mut FrameCtx,
    renderer: &dyn SceneRenderer<ContentKey = SceneContentKey>,
    lighting: &dyn RenderComponent,
    content: &Viewer3dSceneCtx,
    final_target: &RenderTargetView,
    current_camera_view_projection_inv: Mat4<f32>,
    reversed_depth: bool,
    opaque_lighting: LightingTechniqueKind,
    deferred_mat_supports: &DeferLightingMaterialRegistry,
  ) {
    self
      .reproject
      .update(ctx, current_camera_view_projection_inv);

    self.post.upload_with_diff(&ctx.gpu.queue);

    let camera = CameraRenderSource::Scene(content.main_camera);

    let mut msaa_color = attachment().sample_count(4).request(ctx);
    let mut msaa_depth = depth_attachment().sample_count(4).request(ctx);
    let mut widgets_result = attachment().request(ctx);

    let main_camera_gpu = renderer
      .get_camera_gpu()
      .make_component(content.main_camera)
      .unwrap();

    let mut widget_scene_content = renderer.extract_and_make_pass_content(
      SceneContentKey { transparent: false },
      content.widget_scene,
      camera,
      ctx,
      &(),
    );

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(
        msaa_depth.write(),
        clear(if reversed_depth { 0. } else { 1. }),
      )
      .resolve_to(widgets_result.write())
      .render_ctx(ctx)
      .by(&mut super::axis::DrawWorldAxis {
        data: &self.axis,
        reversed_depth,
        camera: main_camera_gpu.as_ref(),
      })
      .by(&mut widget_scene_content);

    let mut highlight_compose = (content.selected_target.is_some()).then(|| {
      let masked_content = renderer.render_models(
        Box::new(IteratorAsHostRenderBatch(content.selected_target)),
        CameraRenderSource::Scene(content.main_camera),
        &HighLightMaskDispatcher,
        ctx,
      );
      self.highlight.draw(ctx, masked_content)
    });

    let taa_content = SceneCameraTAAContent {
      queue: &ctx.gpu.queue,
      camera: content.main_camera,
      renderer,
      f: |ctx: &mut FrameCtx| {
        let mut scene_result = attachment().request(ctx);
        let mut g_buffer = FrameGeometryBuffer::new(ctx);

        let (color_ops, depth_ops) = renderer.init_clear(content.scene);
        let key = SceneContentKey { transparent: false };

        let g_buffer_base_writer = FrameGeometryBufferPassEncoder {
          normal: 2,
          entity_id: 1,
        };

        let material_writer = FrameGeneralMaterialBufferEncoder {
          indices: FrameGeneralMaterialChannelIndices::from_idx(3),
          materials: deferred_mat_supports,
        };

        let scene_pass_dispatcher = match opaque_lighting {
          LightingTechniqueKind::Forward => {
            &RenderArray([&g_buffer_base_writer as &dyn RenderComponent, lighting])
              as &dyn RenderComponent
          }
          LightingTechniqueKind::DeferLighting => &RenderArray([
            &g_buffer_base_writer as &dyn RenderComponent,
            &material_writer,
            lighting,
          ]) as &dyn RenderComponent,
        };

        let mut main_scene_content = renderer.extract_and_make_pass_content(
          key,
          content.scene,
          CameraRenderSource::Scene(content.main_camera),
          ctx,
          scene_pass_dispatcher,
        );

        let mut background = renderer.render_background(
          content.scene,
          CameraRenderSource::Scene(content.main_camera),
        );

        let _span = span!(Level::INFO, "main scene content encode pass");
        let pass_base = pass("scene")
          .with_color(scene_result.write(), color_ops)
          .with_depth(g_buffer.depth.write(), depth_ops)
          .with_color(g_buffer.entity_id.write(), clear(ID_BACKGROUND))
          .with_color(g_buffer.normal.write(), clear(all_zero()));

        match opaque_lighting {
          LightingTechniqueKind::Forward => {
            pass_base
              .render_ctx(ctx)
              // the following pass will check depth to decide if pixel is background,
              // so miss overwrite other channel is not a problem here
              .by(&mut background)
              .by(&mut main_scene_content);
          }
          LightingTechniqueKind::DeferLighting => {
            let mut m_buffer = FrameGeneralMaterialBuffer::new(ctx);

            pass_base
              .with_color(m_buffer.material_type_id.write(), clear(all_zero()))
              .with_color(m_buffer.channel_a.write(), clear(all_zero()))
              .with_color(m_buffer.channel_b.write(), clear(all_zero()))
              .with_color(m_buffer.channel_c.write(), clear(all_zero()))
              .render_ctx(ctx)
              // ditto
              .by(&mut background)
              .by(&mut main_scene_content);
          }
        }

        // this must a separate pass, because the id buffer should not be written.
        pass("grid_ground")
          .with_color(scene_result.write(), load())
          .with_depth(g_buffer.depth.write(), load())
          .render_ctx(ctx)
          .by(&mut GridGround {
            plane: &self.ground,
            shading: &self.grid,
            camera: main_camera_gpu.as_ref(),
            reversed_depth,
          });

        if self.enable_ssao {
          let ao = self.ssao.draw(
            ctx,
            &g_buffer.depth,
            &self.reproject.reproject,
            reversed_depth,
          );

          pass("ao blend to scene")
            .with_color(scene_result.write(), load())
            .render_ctx(ctx)
            .by(&mut copy_frame(
              ao.read_into(),
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

    let mut scene_msaa_widgets = copy_frame(
      widgets_result.read_into(),
      BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
    );

    let mut compose = pass("compose-all")
      .with_color(final_target.clone(), load())
      .render_ctx(ctx)
      .by(
        &mut PostProcess {
          input: taa_result.read(),
          config: &self.post,
        }
        .draw_quad(),
      )
      .by(&mut highlight_compose);

    if self.enable_outline {
      // should we draw outline on taa buffer?
      compose = compose.by(
        &mut OutlineComputer {
          source: &ViewerOutlineSourceProvider {
            g_buffer: &g_buffer,
            reproject: &self.reproject.reproject,
          },
        }
        .draw_quad_with_blend(BlendState::ALPHA_BLENDING.into()),
      );
    }

    compose.by(&mut scene_msaa_widgets);
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
  F: FnOnce(&mut FrameCtx) -> (NewTAAFrameSample, R),
{
  fn set_jitter(&mut self, next_jitter: Vec2<f32>) {
    let cameras = self.renderer.get_camera_gpu();
    cameras.setup_camera_jitter(self.camera, next_jitter, self.queue);
  }

  fn render(self, ctx: &mut FrameCtx) -> (NewTAAFrameSample, R) {
    (self.f)(ctx)
  }
}
