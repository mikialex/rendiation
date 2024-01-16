use std::task::Context;

use rendiation_scene_core::*;
use rendiation_webgpu::*;

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

/// Null adaptor is used to disable or partially disable the rendering.
/// for example disable the fallback behavior in gpu driven rendering in development case.
pub struct NullSceneRenderingAdaptor;

impl SceneRenderingAdaptorBase for NullSceneRenderingAdaptor {
  fn build(_: Scene) -> Self {
    Self
  }
  fn poll_update(&mut self, _: &mut Context) {}
}
impl SceneRasterRenderingAdaptor for NullSceneRenderingAdaptor {
  type DrawTask = ();
  fn create_task(_: &SceneCamera) -> Self::DrawTask {}
  fn render_task_on_frame(&self, _: &mut FrameCtx, _: Self::DrawTask, _: &Attachment) {}
}
