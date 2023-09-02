use rendiation_webgpu::*;

use crate::*;

pub struct WebGPUCanvasRenderer {
  device: GPUDevice,
  queue: GPUQueue,
}

impl TriangulationBasedRendererImpl for WebGPUCanvasRenderer {
  type Image = RenderTargetView;

  fn render(&mut self, target: &Self::Image, content: &GraphicsRepresentation) {
    self.device.create_encoder();
  }
}
