use crate::*;

mod axis;
pub use axis::*;

pub fn draw_widgets(
  ctx: &mut FrameCtx,
  renderer: &dyn SceneRenderer,
  extractor: &ViewerBatchExtractor,
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
    let msaa_color = attachment().sample_count(4).request(ctx);
    let msaa_depth = depth_attachment().sample_count(4).request(ctx);

    pass("scene-widgets-msaa")
      .with_color(&msaa_color, clear_and_store(all_zero()))
      .with_depth(
        &msaa_depth,
        clear_and_store(if reversed_depth { 0. } else { 1. }),
        load_and_store(),
      )
      .resolve_to(&widgets_result)
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
