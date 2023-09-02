use rendiation_shader_api::GraphicsShaderProvider;
use rendiation_webgpu::*;

use crate::*;

pub struct WebGPUCanvasRenderer {
  device: GPUDevice,
  queue: GPUQueue,
}

impl TriangulationBasedRendererImpl for WebGPUCanvasRenderer {
  type Image = RenderTargetView;

  fn render(&mut self, target: &Self::Image, content: &GraphicsRepresentation) {
    let encoder = self.device.create_encoder();

    let metadata_buffer = create_gpu_readonly_storage(content.object_meta.as_slice(), &self.device);
    let vertex_buffer = bytemuck::cast_slice(&content.vertices);
    let vertex_buffer = create_gpu_buffer(vertex_buffer, BufferUsages::VERTEX, &self.device);
    let index_buffer = bytemuck::cast_slice(&content.indices);
    let index_buffer = create_gpu_buffer(index_buffer, BufferUsages::INDEX, &self.device);

    // encode.

    // pass("gui")
    //   .with_color(target.clone(), load())
    //   .render(todo!())
    //   .by(renderable);

    self.queue.submit(Some(encoder.finish()));
  }
}

// struct UIContextGPU {
//   texture_atlas: GPUTextureView,
//   font_atlas: GPUTextureView,
// }

// struct UIPrimitivesGPU {
//   vertex_buffer: GPUBuffer,
//   index_buffer: GPUBuffer,
//   metadata_buffer: GPUBuffer,
// }

// impl GraphicsShaderProvider for UIPrimitivesGPU {
//   fn build(
//     &self,
//     builder: &mut rendiation_shader_api::ShaderRenderPipelineBuilder,
//   ) -> Result<(), rendiation_shader_api::ShaderBuildError> {
//     builder.vertex(|shader, binding| {
//       //
//       Ok(())
//     })
//   }
// }
