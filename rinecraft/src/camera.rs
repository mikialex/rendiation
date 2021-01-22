use rendiation_ral::UniformHandle;
use rendiation_render_entity::Camera;
use rendiation_shader_library::transform::CameraTransform;
use rendiation_webgpu::WebGPU;

pub struct RinecraftCamera {
  camera: Camera,
  gpu: UniformHandle<WebGPU, CameraTransform>,
}

impl RinecraftCamera {
  pub fn new() -> Self {
    //
  }
}
