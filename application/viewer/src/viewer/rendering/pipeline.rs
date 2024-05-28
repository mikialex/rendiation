use rendiation_algebra::*;
use rendiation_texture_gpu_process::*;
use rendiation_webgpu::*;

use super::ScreenChannelDebugger;
use crate::*;

pub struct ViewerPipeline {
  highlight: HighLighter,
  reproject: GPUReprojectInfo,
  taa: TAA,
  pub enable_ssao: bool,
  ssao: SSAO,
  _blur: CrossBlurData,
  pub enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  tonemap: ToneMap,
}

impl ViewerPipeline {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      _blur: CrossBlurData::new(gpu),
      reproject: GPUReprojectInfo::new(gpu),
      taa: TAA::new(),
      enable_ssao: true,
      ssao: SSAO::new(gpu),
      enable_channel_debugger: false,
      channel_debugger: ScreenChannelDebugger::default_useful(),
      tonemap: ToneMap::new(gpu),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    ui.checkbox(&mut self.enable_ssao, "enable ssao");
    ui.checkbox(&mut self.enable_channel_debugger, "enable channel debug");
  }
}

impl ViewerPipeline {
  pub fn render(
    &mut self,
    ctx: &mut FrameCtx,
    _cx: &mut Context,
    renderer: &dyn SceneRenderer,
    content: &Viewer3dSceneCtx,
    final_target: &RenderTargetView,
  ) {
    // let mut mip_gen = scene.resources.bindable_ctx.gpu.mipmap_gen.borrow_mut();
    // mip_gen.flush_mipmap_gen_request(ctx);
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

    let _ = pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render_ctx(ctx);

    let highlight_compose = (content.selected_target.is_some()).then(|| {
      let masked_content = highlight(
        content.selected_target.iter().cloned(),
        content.main_camera,
        renderer,
      );
      self.highlight.draw(ctx, masked_content)
    });

    let taa_content = SceneCameraTAAContent {
      gpu: ctx.gpu,
      scene: content,
      f: |ctx: &mut FrameCtx| {
        let mut scene_result = attachment().request(ctx);
        let mut scene_depth = depth_attachment().request(ctx);

        let current_camera_view_projection_inv = todo!();

        self
          .reproject
          .update(ctx, current_camera_view_projection_inv);

        let ao = self.enable_ssao.then(|| {
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

        // these pass will get correct gpu camera?
        let (color_ops, depth_ops) = renderer.init_clear(content.scene);
        // todo light dispatcher
        let main_scene_content =
          renderer.make_pass_content(content.scene, content.main_camera, &(), ctx);
        pass("scene")
          .with_color(scene_result.write(), color_ops)
          .with_depth(scene_depth.write(), depth_ops)
          .render_ctx(ctx)
          .by(main_scene_content.as_mut())
          // .by(scene.by_main_camera_and_self(&mut s.ground)) // transparent, should go after
          // opaque
          .by(ao);

        NewTAAFrameSample {
          new_color: scene_result,
          new_depth: scene_depth,
        }
      },
    };

    let taa_result = self
      .taa
      .render_aa_content(taa_content, ctx, &self.reproject);

    let main_scene_content = copy_frame(taa_result.read(), None);

    let scene_msaa_widgets = copy_frame(
      widgets_result.read_into(),
      BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
    );

    pass("compose-all")
      .with_color(final_target.clone(), load())
      .render_ctx(ctx)
      .by(main_scene_content)
      .by(highlight_compose)
      .by(scene_msaa_widgets);
  }
}

pub struct HighLightDrawMaskTask<'a, T> {
  objects: Option<T>,
  renderer: &'a dyn SceneRenderer,
  camera: EntityHandle<SceneCameraEntity>,
}

pub fn highlight<T>(
  objects: T,
  camera: EntityHandle<SceneCameraEntity>,
  renderer: &dyn SceneRenderer,
) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask {
    objects: Some(objects),
    camera,
    renderer,
  }
}

impl<'a, T> PassContent for HighLightDrawMaskTask<'a, T>
where
  T: Iterator<Item = EntityHandle<SceneModelEntity>>,
{
  fn render(&mut self, pass: &mut FrameRenderPass) {
    if let Some(mut objects) = self.objects.take() {
      self.renderer.render_reorderable_models(
        &mut objects,
        self.camera,
        &HighLightMaskDispatcher,
        &mut pass.ctx,
        self.renderer.get_scene_model_cx(),
      );
    }
  }
}

struct SceneCameraTAAContent<'a, F> {
  gpu: &'a GPU,
  scene: &'a Viewer3dSceneCtx,
  f: F,
}

impl<'a, F> TAAContent for SceneCameraTAAContent<'a, F>
where
  F: FnOnce(&mut FrameCtx) -> NewTAAFrameSample,
{
  fn set_jitter(&mut self, next_jitter: Vec2<f32>) {
    todo!()
    // let mut cameras = self.scene.scene_resources.cameras.write().unwrap();
    // let camera_gpu = cameras.get_camera_gpu_mut(self.camera).unwrap();

    // camera_gpu
    //   .ubo
    //   .mutate(|uniform| uniform.jitter_normalized = next_jitter)
    //   .upload(&self.gpu.queue);
  }

  fn render(self, ctx: &mut FrameCtx) -> NewTAAFrameSample {
    (self.f)(ctx)
  }
}
