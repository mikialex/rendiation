use crate::*;

pub struct GPUReprojectInfo {
  pub reproject: UniformBufferDataView<ReprojectInfo>,
}

impl GPUReprojectInfo {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      reproject: UniformBufferDataView::create_default(&gpu.device),
    }
  }
  pub fn update(&self, ctx: &mut FrameCtx, current_mvp_inv: Mat4<f32>) {
    self.reproject.mutate(|d| {
      d.previous_camera_view_projection_inv = d.current_camera_view_projection_inv;
      d.previous_camera_view_projection = d.current_camera_view_projection;
      d.current_camera_view_projection_inv = current_mvp_inv;
      d.current_camera_view_projection = current_mvp_inv.inverse_or_identity();
    });

    self.reproject.upload(&ctx.gpu.queue);
    // make sure the reproject is correctly updated in gpu, is this required?
    ctx.make_submit();
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct ReprojectInfo {
  pub current_camera_view_projection: Mat4<f32>,
  pub current_camera_view_projection_inv: Mat4<f32>,
  pub previous_camera_view_projection: Mat4<f32>,
  pub previous_camera_view_projection_inv: Mat4<f32>,
}
