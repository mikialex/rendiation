use crate::*;

mod hierarchy_conservative_depth;

struct DeviceOcclusionCulling {
  last_frame_visibility: StorageBuffer<[bool]>,
}

pub trait DeviceOcclusionCullingContent {
  type DrawTask;

  fn generate_draw_task_of_last_frame_visible_objects(
    &self,
    ctx: &mut FrameCtx,
    last_frame_visibility: &StorageBuffer<[bool]>,
  ) -> Self::DrawTask;

  fn compute_current_visibility_and_the_draw_content_of_current_rest_visible(
    &self,
    ctx: &mut FrameCtx,
    frame_visibility: &StorageBuffer<[bool]>,
  ) -> Self::DrawTask;

  fn render_task_on_frame(self, ctx: &mut FrameCtx, task: Self::DrawTask, target: &Attachment);
}

impl DeviceOcclusionCulling {
  pub fn execute(
    &self,
    ctx: &mut FrameCtx,
    content: impl DeviceOcclusionCullingContent,
    frame: Attachment,
  ) -> Attachment {
    let last_frame_visible_draw_task =
      content.render_last_frame_visible(ctx, &self.last_frame_visibility);

    content.render_task_on_frame(ctx, last_frame_visible_draw_task, &frame);

    let h_depth = generate_h_depth(frame);

    let this_frame_new_visible_draw_task = content
      .compute_current_visibility_and_the_draw_content_of_current_rest_visible(
        ctx,
        &self.last_frame_visibility,
      );

    content.render_task_on_frame(ctx, this_frame_new_visible_draw_task, &frame);

    frame
  }
}
