use rendiation_algebra::*;
use rendiation_infinity_primitive::*;
use rendiation_texture_gpu_process::*;
use rendiation_webgpu::*;

use super::{GridEffect, GridGround};
use crate::*;

pub struct ViewerFrameLogic {
  highlight: HighLighter,
  reproject: GPUReprojectInfo,
  taa: TAA,
  pub enable_ssao: bool,
  ssao: SSAO,
  _blur: CrossBlurData,
  ground: UniformBufferCachedDataView<ShaderPlane>,
  grid: UniformBufferCachedDataView<GridEffect>,
  post: UniformBufferCachedDataView<PostEffects>,
}

impl ViewerFrameLogic {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      _blur: CrossBlurData::new(gpu),
      reproject: GPUReprojectInfo::new(gpu),
      taa: TAA::new(),
      enable_ssao: true,
      ssao: SSAO::new(gpu),
      ground: UniformBufferCachedDataView::create(&gpu.device, ShaderPlane::ground_like()),
      grid: UniformBufferCachedDataView::create_default(&gpu.device),
      post: UniformBufferCachedDataView::create_default(&gpu.device),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    ui.checkbox(&mut self.enable_ssao, "enable ssao");

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
  ) {
    self
      .reproject
      .update(ctx, current_camera_view_projection_inv);

    self.post.upload_with_diff(&ctx.gpu.queue);

    // let mut single_proj_sys = scene
    //   .scene_resources
    //   .shadows
    //   .single_proj_sys
    //   .write()
    //   .unwrap();
    // single_proj_sys.update_depth_maps(ctx, scene);
    // drop(single_proj_sys);

    let mut msaa_color = attachment().sample_count(4).request(ctx);
    let mut msaa_depth = depth_attachment().sample_count(4).request(ctx);
    let mut widgets_result = attachment().request(ctx);

    let main_camera_gpu = renderer
      .get_camera_gpu()
      .make_dep_component(content.main_camera)
      .unwrap();

    let mut widget_scene_content = renderer.extract_and_make_pass_content(
      SceneContentKey { transparent: false },
      content.widget_scene,
      content.main_camera,
      ctx,
      &(),
    );

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render_ctx(ctx)
      .by(&mut widget_scene_content);

    let mut highlight_compose = (content.selected_target.is_some()).then(|| {
      let masked_content = renderer.render_models(
        Box::new(IteratorAsHostRenderBatch(content.selected_target)),
        content.main_camera,
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

        let mut ao = self.enable_ssao.then(|| {
          let ao = self.ssao.draw(ctx, &scene_depth, &self.reproject.reproject);
          copy_frame(
            ao.read_into(),
            BlendState {
              color: BlendComponent {
                src_factor: BlendFactor::Dst,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
              },
              alpha: BlendComponent::REPLACE,
            }
            .into(),
          )
        });

        let (color_ops, depth_ops) = renderer.init_clear(content.scene);
        let key = SceneContentKey { transparent: false };
        let mut main_scene_content = renderer.extract_and_make_pass_content(
          key,
          content.scene,
          content.main_camera,
          ctx,
          lighting,
        );

        let _span = span!(Level::INFO, "main scene content encode pass");
        pass("scene")
          .with_color(scene_result.write(), color_ops)
          .with_depth(scene_depth.write(), depth_ops)
          .render_ctx(ctx)
          .by(&mut main_scene_content)
          .by(&mut GridGround {
            plane: &self.ground,
            shading: &self.grid,
            camera: main_camera_gpu.as_ref(),
          })
          .by(&mut ao);

        NewTAAFrameSample {
          new_color: scene_result,
          new_depth: scene_depth,
        }
      },
    };

    let taa_result = self
      .taa
      .render_aa_content(taa_content, ctx, &self.reproject);

    let mut scene_msaa_widgets = copy_frame(
      widgets_result.read_into(),
      BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
    );

    pass("compose-all")
      .with_color(final_target.clone(), load())
      .render_ctx(ctx)
      .by(
        &mut PostProcess {
          input: taa_result.read(),
          config: &self.post,
        }
        .draw_quad(),
      )
      .by(&mut highlight_compose)
      .by(&mut scene_msaa_widgets);
  }
}

struct SceneCameraTAAContent<'a, F> {
  renderer: &'a dyn SceneRenderer<ContentKey = SceneContentKey>,
  camera: EntityHandle<SceneCameraEntity>,
  queue: &'a GPUQueue,
  f: F,
}

impl<'a, F> TAAContent for SceneCameraTAAContent<'a, F>
where
  F: FnOnce(&mut FrameCtx) -> NewTAAFrameSample,
{
  fn set_jitter(&mut self, next_jitter: Vec2<f32>) {
    let cameras = self.renderer.get_camera_gpu();
    cameras.setup_camera_jitter(self.camera, next_jitter, self.queue);
  }

  fn render(self, ctx: &mut FrameCtx) -> NewTAAFrameSample {
    (self.f)(ctx)
  }
}
