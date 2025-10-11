use crate::*;

mod axis;
pub use axis::*;

pub fn draw_widgets(
  ctx: &mut FrameCtx,
  renderer: &dyn SceneRenderer,
  extractor: &DefaultSceneBatchExtractor,
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
    ctx,
  );

  let mut widget_scene_content = renderer.make_scene_batch_pass_content(
    batch,
    main_camera_gpu,
    &DefaultDisplayWriter {
      write_channel_index: 0,
    },
    ctx,
  );

  #[cfg(not(feature = "webgl"))]
  let enable_msaa = true;

  // msaa can be enable in webgl, if we restrict the texture usage to attachment only
  #[cfg(feature = "webgl")]
  let enable_msaa = false;

  if enable_msaa {
    let widgets_result = attachment().request(ctx);
    let msaa_color = attachment().sample_count(4).request(ctx);
    let msaa_depth = depth_attachment().sample_count(4).request(ctx);

    pass("scene-widgets-msaa")
      .with_color(&msaa_color, clear_and_store(all_zero()))
      .with_depth(
        &msaa_depth,
        clear_and_store(if reversed_depth { 0. } else { 1. }),
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
