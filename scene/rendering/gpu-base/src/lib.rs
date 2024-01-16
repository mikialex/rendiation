pub trait SceneRenderingAdaptorBase {
  /// self will react to scene change, and update by polling api
  fn build(scene: Scene) -> Self;
  fn poll_update(&mut self, cx: &mut Context);
}

pub trait SceneRasterRenderingAdaptor: SceneRenderingAdaptorBase {
  type DrawTask;

  /// should contains frustum culling and lod select
  fn create_task(camera: &SceneCamera) -> Self::DrawTask;

  fn render_task_on_frame(&self, ctx: &mut FrameCtx, task: Self::DrawTask, target: &Attachment);
}
