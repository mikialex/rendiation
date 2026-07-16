use rendiation_occ_style_draw_control::*;
use rendiation_texture_gpu_process::copy_frame;
use rendiation_webgpu::*;

use crate::*;

pub struct TopMostStandaloneDraw {
  pub scene: EntityHandle<SceneEntity>,
  pub reverse_z: bool,
}

impl TopMostStandaloneDraw {
  fn use_draw_content_on_post_frame_impl(
    &self,
    ctx: &mut FrameCtx,
    renderer: &ViewerRendererInstance,
    lighting: &LightingRenderingCx,
    camera: EntityHandle<SceneCameraEntity>,
    target: &RenderTargetView,
  ) -> Option<()> {
    let batch_extractor = renderer
      .batch_extractor
      .as_any()
      .downcast_ref::<ViewerBatchExtractor>()?;

    let batch = if let Some(extractor) = batch_extractor.indirect_extractor.as_ref() {
      let extractor = extractor
        .as_any()
        .downcast_ref::<LockReadGuardHolder<OccStyleOrderControlSceneBatchExtractor>>()?;
      extractor.get_top_most_layer(self.scene)
    } else {
      let extractor = batch_extractor
        .default_extractor
        .as_any()
        .downcast_ref::<OccStyleOrderControlSceneBatchExtractorGles>()?;

      extractor.get_top_most_layer(self.scene, renderer.raster_scene_renderer.as_ref())
    };

    let forward_lighting = lighting
      .lighting
      .get_scene_forward_lighting_component(self.scene, camera);

    let color_writer = DefaultDisplayWriter {
      write_channel_index: 0,
    };

    let pass_dispatcher = &RenderArray([&color_writer as &dyn RenderComponent, &forward_lighting])
      as &dyn RenderComponent;

    let main_camera_gpu = renderer.camera.make_component(camera)?;

    let mut top_most_scene_content = renderer
      .raster_scene_renderer
      .make_scene_batch_pass_content(batch, &main_camera_gpu, pass_dispatcher, ctx);

    // should we consider msaa config?
    let top_most_result = attachment().request(ctx);
    let msaa_color = attachment().sample_count(MSAA_SAMPLE_COUNT).request(ctx);
    let msaa_depth = depth_attachment()
      .sample_count(MSAA_SAMPLE_COUNT)
      .request(ctx);

    pass("scene_top_most_mass")
      .with_color_and_resolve_target(&msaa_color, clear_and_store(all_zero()), &top_most_result)
      .with_depth(
        &msaa_depth,
        clear_and_store(if self.reverse_z { 0. } else { 1. }),
        load_and_store(),
      )
      .render_ctx(ctx)
      .by(&mut top_most_scene_content);

    let mut copy = copy_frame(
      top_most_result,
      BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(), // is this good?
    );

    pass("copy_top_most_to_target")
      .with_color(target, load_and_store())
      .render_ctx(ctx)
      .by(&mut copy);

    Some(())
  }
}

impl ViewerFrameRenderingExtension for TopMostStandaloneDraw {
  fn use_draw_content_on_post_frame(
    &mut self,
    frame: &mut FrameCtx,
    renderer: &ViewerRendererInstance,
    lighting: &LightingRenderingCx,
    camera: EntityHandle<SceneCameraEntity>,
    target: &RenderTargetView,
  ) {
    if self
      .use_draw_content_on_post_frame_impl(frame, renderer, lighting, camera, target)
      .is_none()
    {
      log::warn!("failed to draw topmost layer")
    }
  }
}
