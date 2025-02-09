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
  pub enable_ssao: bool,
  pub enable_outline: bool,
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

    ui.collapsing("vignette", |ui| {
      self.post.mutate(|post| {
        let mut enabled: bool = post.enable_vignette.into();
        ui.checkbox(&mut enabled, "enabled");
        post.enable_vignette = enabled.into();

        ui.add(
          egui::Slider::new(&mut post.vignette.radius, 0.0..=1.0)
            .step_by(0.05)
            .text("radius"),
        );
        ui.add(
          egui::Slider::new(&mut post.vignette.feather, 0.0..=1.0)
            .step_by(0.05)
            .text("feather"),
        );
        ui.add(
          egui::Slider::new(&mut post.vignette.mid_point, 0.0..=1.0)
            .step_by(0.05)
            .text("mid_point"),
        );
      });
    });

    self.post.mutate(|post| {
      let mut enabled: bool = post.enable_chromatic_aberration.into();
      ui.checkbox(&mut enabled, "enable_chromatic_aberration");
      post.enable_chromatic_aberration = enabled.into();
    });
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
        let mut scene_depth = depth_attachment().request(ctx);

        let (color_ops, depth_ops) = renderer.init_clear(content.scene);
        let key = SceneContentKey { transparent: false };

        let scene_pass_dispatcher = if self.enable_outline {
          &RenderArray([
            &EntityIdWriter { id_channel_idx: 1 } as &dyn RenderComponent,
            &NormalWriter { id_channel_idx: 2 } as &dyn RenderComponent,
            lighting,
          ])
        } else {
          lighting
        };

        let mut main_scene_content = renderer.extract_and_make_pass_content(
          key,
          content.scene,
          CameraRenderSource::Scene(content.main_camera),
          ctx,
          scene_pass_dispatcher,
        );

        // todo create in branch
        let mut id_buffer = attachment().format(TextureFormat::R32Uint).request(ctx);
        let mut normal_buffer = attachment()
          .format(TextureFormat::Rgb10a2Unorm)
          .request(ctx);

        let id_background = rendiation_webgpu::Color {
          r: u32::MAX as f64,
          g: 0.,
          b: 0.,
          a: 0.,
        };

        let _span = span!(Level::INFO, "main scene content encode pass");
        let mut scene_main_pass_desc = pass("scene")
          .with_color(scene_result.write(), color_ops)
          .with_depth(scene_depth.write(), depth_ops);

        if self.enable_outline {
          scene_main_pass_desc =
            scene_main_pass_desc.with_color(id_buffer.write(), clear(id_background));
          scene_main_pass_desc =
            scene_main_pass_desc.with_color(normal_buffer.write(), clear(all_zero()));
        }

        scene_main_pass_desc
          .render_ctx(ctx)
          .by(&mut renderer.render_background(
            content.scene,
            CameraRenderSource::Scene(content.main_camera),
          ))
          .by(&mut main_scene_content);

        // this must a separate pass, because the id buffer should not be written.
        pass("grid_ground")
          .with_color(scene_result.write(), load())
          .with_depth(scene_depth.write(), load())
          .render_ctx(ctx)
          .by(&mut GridGround {
            plane: &self.ground,
            shading: &self.grid,
            camera: main_camera_gpu.as_ref(),
            reversed_depth,
          });

        if self.enable_ssao {
          let ao = self
            .ssao
            .draw(ctx, &scene_depth, &self.reproject.reproject, reversed_depth);

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
            new_depth: scene_depth,
          },
          (id_buffer, normal_buffer),
        )
      },
    };

    let (taa_result, scene_depth, (id_buffer, normal_buffer)) =
      self
        .taa
        .render_aa_content(taa_content, ctx, &self.reproject);

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
            normal: &normal_buffer,
            depth: &scene_depth,
            ids: &id_buffer,
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
