use rendiation_algebra::Vec2;
use rendiation_webgpu::PipelineBuilder;

pub fn full_screen_vertex_shader(builder: &mut PipelineBuilder) {
  builder
    .include_vertex_entry(
      "
      [[stage(vertex)]]
      fn vs_main_full_screen(
        [[builtin(vertex_index)]] vertex_index: u32,
      ) -> VertexOutput {
        var out: VertexOutput;

        var left: f32 = -1.0;
        var right: f32 = 1.0;
        var top: f32 = 1.0;
        var bottom: f32 = -1.0;
        var depth: f32 = 0.0;

        switch (i32(vertex_index)) {
          case 0: {
            out.position = vec4<f32>(left, top, depth, 1.);
            out.uv = vec2<f32>(0., 0.);
          }
          case 1: {
            out.position = vec4<f32>(right, top, depth, 1.);
            out.uv = vec2<f32>(1., 0.);
          }
          case 2: {
            out.position = vec4<f32>(left, bottom, depth, 1.);
            out.uv = vec2<f32>(0., 1.);
          }
          case 3: {
            out.position = vec4<f32>(right, bottom, depth, 1.);
            out.uv = vec2<f32>(1., 1.);
          }
        }

        return out;
      }
    ",
    )
    .use_vertex_entry("vs_main_full_screen");
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
}
