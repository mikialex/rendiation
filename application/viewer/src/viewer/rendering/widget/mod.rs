use crate::*;

mod axis;
pub use axis::*;

pub fn use_widget_draw(ctx: &mut Viewer3dRenderingCx) {
  // pub axis: WorldCoordinateAxis,
}

fn draw_widgets(
  ctx: &mut FrameCtx,
  renderer: &dyn SceneRenderer<ContentKey = SceneContentKey>,
  widget_scene: EntityHandle<SceneEntity>,
  reversed_depth: bool,
  camera: EntityHandle<SceneCameraEntity>,
  axis: &WorldCoordinateAxis,
) -> RenderTargetView {
  let main_camera_gpu = renderer.get_camera_gpu().make_component(camera).unwrap();
  let camera = CameraRenderSource::Scene(camera);

  let mut widget_scene_content = renderer.extract_and_make_pass_content(
    SceneContentKey {
      only_alpha_blend_objects: None,
    },
    widget_scene,
    camera,
    ctx,
    &DefaultDisplayWriter,
  );

  let widgets_result = attachment().request(ctx);
  let msaa_color = attachment().sample_count(4).request(ctx);
  let msaa_depth = depth_attachment().sample_count(4).request(ctx);

  pass("scene-widgets")
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
      camera: main_camera_gpu.as_ref(),
    })
    .by(&mut widget_scene_content);

  widgets_result
}
