use crate::*;

pub struct GPUReprojectInfo {
  pub reproject: UniformBufferCachedDataView<ReprojectInfo>,
  pub camera_position: Vec3<f64>,
}

impl GPUReprojectInfo {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      reproject: UniformBufferCachedDataView::create_default(&gpu.device),
      camera_position: Vec3::zero(),
    }
  }
  pub fn update(
    &mut self,
    ctx: &mut FrameCtx,
    current_vp_no_translation: Mat4<f32>,
    camera_position: Vec3<f64>,
  ) {
    let current_mvp_no_translation_inv = current_vp_no_translation.inverse_or_identity();
    let delta = self.camera_position - camera_position;
    self.camera_position = camera_position;
    self.reproject.mutate(|d| {
      d.previous_camera_view_projection_inv = d.current_camera_view_projection_inv;
      d.previous_camera_view_projection = d.current_camera_view_projection;
      d.current_camera_view_projection_inv = current_mvp_no_translation_inv;
      d.current_camera_view_projection = current_vp_no_translation;
      d.camera_position_delta = delta.into_f32();
    });

    self.reproject.upload(&ctx.gpu.queue);
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
  // from old to new
  pub camera_position_delta: Vec3<f32>,
}
