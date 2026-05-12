use crate::*;

mod axis;
pub use axis::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_texture_gpu_process::copy_frame;

pub struct ViewerAppFrameRenderingExtension<'a> {
  pub widget_scene: EntityHandle<SceneEntity>,
  pub axis: &'a WorldCoordinateAxis,
}

impl<'a> ViewerFrameRenderingExtension for ViewerAppFrameRenderingExtension<'a> {
  fn use_draw_content_on_post_frame(
    &mut self,
    ctx: &mut FrameCtx,
    renderer: &ViewerRendererInstance,
    camera: EntityHandle<SceneCameraEntity>,
    target: &RenderTargetView,
  ) {
    let main_camera_gpu = renderer.camera.make_component(camera).unwrap();

    let widgets_result = draw_widgets(
      ctx,
      renderer.raster_scene_renderer.as_ref(),
      renderer.batch_extractor.as_ref(),
      self.widget_scene,
      renderer.reversed_depth,
      &main_camera_gpu,
      &self.axis,
    );
    let mut copy_scene_msaa_widgets = copy_frame(
      widgets_result,
      BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
    );
    pass("copy_scene_msaa_widgets")
      .with_color(&target, load_and_store())
      .render_ctx(ctx)
      .by(&mut copy_scene_msaa_widgets);
  }
}

pub fn draw_widgets(
  ctx: &mut FrameCtx,
  renderer: &dyn SceneRenderer,
  extractor: &dyn SceneBatchBasicExtractAbility,
  widget_scene: EntityHandle<SceneEntity>,
  reversed_depth: bool,
  main_camera_gpu: &dyn RenderComponent,
  axis: &WorldCoordinateAxis,
) -> RenderTargetView {
  let batch = extractor.extract_scene_batch(
    widget_scene,
    SceneContentKey {
      only_alpha_blend_objects: None,
    },
    renderer,
  );

  let mut widget_scene_content = renderer.make_scene_batch_pass_content(
    batch,
    main_camera_gpu,
    &DefaultDisplayWriter {
      write_channel_index: 0,
    },
    ctx,
  );

  // msaa can be enabled in webgl, if we restrict the texture usage to attachment only
  #[allow(clippy::needless_bool)]
  let enable_msaa = if ctx.gpu.info().adaptor_info.backend == Backend::Gl {
    #[cfg(feature = "support-webgl")]
    {
      false
    }
    #[cfg(not(feature = "support-webgl"))]
    {
      true
    }
  } else {
    true
  };

  if enable_msaa {
    let widgets_result = attachment().request(ctx);
    let msaa_color = attachment().sample_count(MSAA_SAMPLE_COUNT).request(ctx);
    let msaa_depth = depth_attachment()
      .sample_count(MSAA_SAMPLE_COUNT)
      .request(ctx);

    pass("scene-widgets-msaa")
      .with_color_and_resolve_target(&msaa_color, clear_and_store(all_zero()), &widgets_result)
      .with_depth(
        &msaa_depth,
        clear_and_store(if reversed_depth { 0. } else { 1. }),
        load_and_store(),
      )
      .render_ctx(ctx)
      .by(&mut DrawWorldAxis {
        data: axis,
        reversed_depth,
        camera: main_camera_gpu,
      })
      .by(&mut widget_scene_content);

    widgets_result
  } else {
    let widgets_result = attachment().request(ctx);
    let depth = depth_attachment().request(ctx);

    pass("scene-widgets-no-msaa")
      .with_color(&widgets_result, clear_and_store(all_zero()))
      .with_depth(
        &depth,
        clear_and_store(if reversed_depth { 0. } else { 1. }),
        load_and_store(),
      )
      .render_ctx(ctx)
      .by(&mut DrawWorldAxis {
        data: axis,
        reversed_depth,
        camera: main_camera_gpu,
      })
      .by(&mut widget_scene_content);

    widgets_result
  }
}
