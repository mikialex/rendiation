use crate::*;

mod hierarchy_conservative_depth;

struct DeviceOcclusionCulling<T> {
  adaptor: T,
  // last_frame_visibility: FastHashMap<SceneCameraId, StorageBuffer<[bool]>>,
}

pub trait DeviceOcclusionCullingContent: SceneRasterRenderingAdaptor {
  fn create_draw_task_of_last_frame_visible_objects(
    &self,
    ctx: &mut FrameCtx,
    camera: &SceneCamera,
    last_frame_visibility: &StorageBuffer<[bool]>,
  ) -> Self::DrawTask;

  // this two things mixed together is to enable the implementor could use a single dispatch
  fn compute_current_all_visibility_and_draw_rest_of_current_visible(
    &self,
    ctx: &mut FrameCtx,
    camera: &SceneCamera,
    frame_visibility: &StorageBuffer<[bool]>,
  ) -> Self::DrawTask;
}

impl<T: DeviceOcclusionCullingContent> SceneRenderingAdaptorBase for DeviceOcclusionCulling<T> {
  fn build(scene: Scene) -> Self {
    todo!()
  }
  fn poll_update(&mut self, cx: &mut Context) {
    todo!()
  }
}

impl<T: DeviceOcclusionCullingContent> SceneRasterRenderingAdaptor for DeviceOcclusionCulling<T> {
  type DrawTask = T::DrawTask;

  /// should contains frustum culling and lod select
  fn create_task(camera: &SceneCamera) -> Self::DrawTask {
    todo!()
  }

  fn render_task_on_frame(&self, ctx: &mut FrameCtx, task: Self::DrawTask, target: &Attachment) {
    todo!()
  }
}

// impl DeviceOcclusionCulling {
//   pub fn execute(
//     &self,
//     ctx: &mut FrameCtx,
//     content: impl DeviceOcclusionCullingContent,
//     frame: Attachment,
//   ) -> Attachment {
//     let last_frame_visible_draw_task =
//       content.create_draw_task_of_last_frame_visible_objects(ctx, &self.last_frame_visibility);

//     content.render_task_on_frame(ctx, last_frame_visible_draw_task, &frame);

//     let h_depth = generate_h_depth(frame);

//     let this_frame_new_visible_draw_task = content
//       .compute_current_all_visibility_and_draw_rest_of_current_visible(
//         ctx,
//         &self.last_frame_visibility,
//       );

//     content.render_task_on_frame(ctx, this_frame_new_visible_draw_task, &frame);

//     frame
//   }
// }
